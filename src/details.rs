use super::identifiable::*;
use crate::changable::Changable;
use crate::change_abs::{AppliedChange, NoopChange};
use crate::changes::FullChanges;
use crate::historic::Historic;
use crate::result::{AlreadyExists, NotFound};
use std::cmp::{Eq, PartialEq};
use std::fmt;
use std::hash;
use std::marker;
use std::mem;
use std::ops;
use std::result::Result as StdResult;
use std::slice;
use DetailsEvent::*;

pub enum DetailsEvent<T>
where
    T: GetId,
    T::IdentifiableType: Owned,
{
    Created(T),
    Updated(T),
    Deleted(Id<<T as GetId>::IdentifiableType>),
}

impl<T> DetailsEvent<T>
where
    Id<T::IdentifiableType>: Clone,
    T: GetId,
    T::IdentifiableType: Owned,
{
    pub fn get_id(&self) -> Option<Id<T::IdentifiableType>> {
        match self {
            Created(x) => Some(x.get_id()),
            Updated(x) => Some(x.get_id()),
            Deleted(id) => Some(id.clone()),
        }
    }

    pub fn merge(&mut self, new: Self) -> EventMergeResult {
        use EventMergeResult::*;

        match (self as &_, new) {
            (Created(_), Updated(now)) => {
                *self = Created(now);
                Combined
            }
            (Updated(_), Updated(now)) => {
                *self = Updated(now);
                Combined
            }
            (Created(_), Deleted(_)) => Annihilated,
            (Updated(_), Deleted(id)) => {
                *self = Deleted(id);
                Combined
            }
            _ => panic!("cannot combine events"),
        }
    }
}

pub enum EventMergeResult {
    Combined,
    Annihilated,
}

impl<T> fmt::Debug for DetailsEvent<T>
where
    T: fmt::Debug + GetId,
    T::IdentifiableType: Owned,
    Id<T::IdentifiableType>: fmt::Debug,
    Id<<T::IdentifiableType as Owned>::OwnerType>: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Created(x) => write!(f, "DbOwnedEvent::Created({:?})", x),
            Updated(x) => write!(f, "DbOwnedEvent::Updated({:?})", x),
            Deleted(x) => write!(f, "DbOwnedEvent::Deleted({:?})", x),
        }
    }
}

impl<T> PartialEq for DetailsEvent<T>
where
    T: PartialEq + GetId,
    T::IdentifiableType: Owned,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Created(x), Created(y)) => x == y,
            (Updated(x), Updated(y)) => x == y,
            (Deleted(x), Deleted(y)) => x == y,
            _ => false,
        }
    }
}

impl<T> Eq for DetailsEvent<T>
where
    T: Eq + GetId,
    T::IdentifiableType: Owned,
{
}

impl<T> Clone for DetailsEvent<T>
where
    T: Clone + GetId,
    T::IdentifiableType: Owned,
    Id<T::IdentifiableType>: Clone,
    Id<<T::IdentifiableType as Owned>::OwnerType>: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Created(x) => Created(x.clone()),
            Updated(x) => Updated(x.clone()),
            Deleted(x) => Deleted(x.clone()),
        }
    }
}

pub struct Details<T, C = FullChanges<DetailsEvent<T>>>
where
    T: GetId,
    T::IdentifiableType: Owned,
{
    inner: Vec<T>,
    complete: bool,
    marker: marker::PhantomData<C>,
}

impl<T, C> Eq for Details<T, C>
where
    T: GetId + Eq,
    T::IdentifiableType: Owned,
{
}

impl<T, C> PartialEq for Details<T, C>
where
    T: GetId + PartialEq,
    T::IdentifiableType: Owned,
{
    fn eq(&self, other: &Self) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl<T, C> fmt::Debug for Details<T, C>
where
    T: GetId + fmt::Debug,
    T::IdentifiableType: Owned,
    Id<T::IdentifiableType>: fmt::Debug,
    Id<<T::IdentifiableType as Owned>::OwnerType>: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}

impl<T, C, I> ops::Index<I> for Details<T, C>
where
    I: slice::SliceIndex<[T]>,
    T: GetId,
    T::IdentifiableType: Owned,
{
    type Output = I::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        &self.inner[index]
    }
}

impl<'a, T, C> IntoIterator for &'a Details<T, C>
where
    T: GetId,
    T::IdentifiableType: Owned,
{
    type Item = &'a T;
    type IntoIter = slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

impl<T, C> Default for Details<T, C>
where
    T: GetId + Clone,
    T::IdentifiableType: Owned,
    Id<T::IdentifiableType>: hash::Hash + Clone,
    Id<<T::IdentifiableType as Owned>::OwnerType>: Clone,
    C: AppliedChange<DetailsEvent<T>>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T, C> Clone for Details<T, C>
where
    T: GetId + Clone,
    T::IdentifiableType: Owned,
    DetailsEvent<T>: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            complete: self.complete,
            marker: self.marker,
        }
    }
}

impl<T, C> Historic for Details<T, C>
where
    T: GetId,
    T::IdentifiableType: Owned,
{
    type EventType = DetailsEvent<T>;
}

impl<T, C> Changable for Details<T, C>
where
    T: GetId,
    T::IdentifiableType: Owned,
    Id<T::IdentifiableType>: hash::Hash + Clone,
    Id<<T::IdentifiableType as Owned>::OwnerType>: Clone,
{
    fn apply(&mut self, event: Self::EventType) -> Self::EventType {
        match event {
            Created(x) => {
                let id = x.get_id();
                self.inner.push(x);
                Deleted(id)
            }
            Updated(x) => {
                let id = x.get_id();
                let pos = self.position_by_id(&id).expect("Dev error: id not found");
                let old = mem::replace(&mut self.inner[pos], x);
                Updated(old)
            }
            Deleted(id) => {
                let pos = self.position_by_id(&id).expect("Dev error: id not found");
                let old = self.inner.remove(pos);
                Created(old)
            }
        }
    }
}

impl<T, C> Details<T, C>
where
    T: GetId,
    T::IdentifiableType: Owned,
{
    pub fn new() -> Self {
        Self {
            inner: Vec::new(),
            complete: true,
            marker: marker::PhantomData,
        }
    }

    fn position_by_id(&self, id: &Id<T::IdentifiableType>) -> Option<usize> {
        self.inner.iter().position(|x| &x.get_id() == id)
    }

    pub fn get<I>(&self, index: I) -> Option<&<I as slice::SliceIndex<[T]>>::Output>
    where
        I: slice::SliceIndex<[T]>,
    {
        self.inner.get(index)
    }
}

impl<T, C> Details<T, C>
where
    C: AppliedChange<DetailsEvent<T>>,
    T: GetId,
    T::IdentifiableType: Owned,
    Id<T::IdentifiableType>: hash::Hash + Clone,
    Id<<T::IdentifiableType as Owned>::OwnerType>: Clone,
{
    pub fn iter(&self) -> slice::Iter<'_, T> {
        self.inner.iter()
    }

    pub fn by_id(&self, id: &Id<T::IdentifiableType>) -> Option<&T> {
        self.find(|x| &x.get_id() == id)
    }

    pub fn find<P>(&self, mut predicate: P) -> Option<&T>
    where
        P: FnMut(&T) -> bool,
    {
        self.inner.iter().find(|x| predicate(x))
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Replaces `criteria` matching items in a collection and returns diff-change
    /// which represents removal, update and creation of items as
    /// necessary
    pub fn set_some<P>(&mut self, mut criteria: P, items: impl IntoIterator<Item = T>) -> C
    where
        T: Eq + fmt::Debug,
        P: FnMut(&T) -> bool,
    {
        let mut changes = Vec::new();

        let mut existing_ids: Vec<_> = self
            .inner
            .iter()
            .filter(|x| criteria(*x))
            .map(GetId::get_id)
            .collect();

        let mut new_ids = Vec::new();

        for x in items {
            new_ids.push(x.get_id());
            if let Some(pos) = self.position_by_id(&x.get_id()) {
                if &x != &self.inner[pos] {
                    changes.push(Updated(x));
                }
            } else {
                changes.push(Created(x));
            }
        }

        let missing_ids = {
            existing_ids.retain(|x| !new_ids.contains(x));
            existing_ids
        };

        for id in missing_ids {
            changes.push(Deleted(id));
        }

        self.applied_many(changes)
    }

    /// Replaces all items in a collection and returns diff-change
    /// which represents removal, update and creation of items as
    /// necessary
    pub fn set_all(&mut self, items: impl IntoIterator<Item = T>) -> C
    where
        T: Eq + fmt::Debug,
    {
        let mut changes = Vec::new();

        let mut existing_ids: Vec<_> = self.inner.iter().map(GetId::get_id).collect();

        let mut new_ids = Vec::new();

        for x in items {
            new_ids.push(x.get_id());
            if let Some(pos) = self.position_by_id(&x.get_id()) {
                if &x != &self.inner[pos] {
                    changes.push(Updated(x));
                }
            } else {
                changes.push(Created(x));
            }
        }

        let missing_ids = {
            existing_ids.retain(|x| !new_ids.contains(x));
            existing_ids
        };

        for id in missing_ids {
            changes.push(Deleted(id));
        }

        self.applied_many(changes)
    }

    pub fn update_or_add(&mut self, item: T) -> C
    where
        T: Eq + fmt::Debug,
        C: NoopChange,
    {
        self.update(item)
            .or_else(|NotFound(x)| self.add_new(x))
            .unwrap()
    }

    /**
     * Updates existing item or returns item back as a Result::Err
     */
    pub fn update(&mut self, item: T) -> StdResult<C, NotFound<T>>
    where
        C: NoopChange,
        T: Eq,
    {
        if let Some(pos) = self.position_by_id(&item.get_id()) {
            if &item == ops::Index::index(self, pos) {
                Ok(C::noop())
            } else {
                Ok(self.applied(Updated(item)))
            }
        } else {
            Err(NotFound(item))
        }
    }

    /**
     * Inserts a new item and returns `Ok(())` if item with the same id does not exist.
     * Returns `Err(item)` if item with the same already exists.
     */
    pub fn add_new(&mut self, item: T) -> StdResult<C, AlreadyExists<T>>
    where
        C: NoopChange,
        T: Eq,
    {
        let id = item.get_id();
        if let None = self.position_by_id(&id) {
            Ok(self.applied(Created(item)))
        } else {
            Err(AlreadyExists(item))
        }
    }

    pub fn remove(&mut self, item: &T) -> StdResult<C, NotFound<Id<T::IdentifiableType>>> {
        let id = item.get_id();
        match self.remove_by_id(&id) {
            Err(_) => Err(NotFound(id)),
            Ok(changes) => Ok(changes),
        }
    }

    pub fn remove_by_id<'a>(
        &mut self,
        id: &'a Id<T::IdentifiableType>,
    ) -> StdResult<C, NotFound<&'a Id<T::IdentifiableType>>> {
        if let Some(_) = self.position_by_id(id) {
            Ok(self.applied(Deleted(id.clone())))
        } else {
            Err(NotFound(id))
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::changes::{FullChange, FullChanges};
    use pretty_assertions::assert_eq;
    use std::cmp::{Eq, PartialEq};
    use std::rc::Rc;
    use Color::*;

    struct TestOwner {
        id: i32,
    }

    impl Identifiable for TestOwner {
        type IdType = i32;

        fn id(&self) -> Id<Self> {
            Id::new(self.id)
        }
    }

    #[derive(Debug, Eq, PartialEq)]
    struct TestEntry {
        owner_id: i32,
        child_id: String,
        name: String,
    }

    impl Identifiable for TestEntry {
        type IdType = String;

        fn id(&self) -> Id<Self> {
            Id::new(self.child_id.clone())
        }
    }

    impl Owned for TestEntry {
        type OwnerType = TestOwner;
    }

    #[derive(Debug)]
    enum Color {
        None,
        Red,
        Green,
        Blue,
    }

    impl From<usize> for Color {
        fn from(value: usize) -> Self {
            use Color::*;

            match value % 4 {
                0 => None,
                1 => Red,
                2 => Green,
                _ => Blue,
            }
        }
    }

    const ANY_NOT_USED_ENTRY_ID: usize = 10000;
    const EXISTING_ID: usize = 0;
    const DELETED_ID: usize = 1;
    const NEW_ID: usize = 2;
    const IGNORED_ID: usize = 3;

    fn colored_id(seed: usize) -> Id<TestEntry> {
        Id::new(raw_colored_id(seed))
    }

    fn raw_colored_id(number: usize) -> String {
        number.to_string()
    }

    fn colored(number: usize, c: Color) -> Rc<TestEntry> {
        TestEntry {
            owner_id: 1,
            child_id: raw_colored_id(number),
            name: format!("{:#?}", c),
        }
        .into()
    }

    type Sut = Details<Rc<TestEntry>>;

    fn setup_existing() -> Sut {
        let mut sut = Details::new();
        sut.update_or_add(colored(ANY_NOT_USED_ENTRY_ID, None));
        sut.update_or_add(colored(EXISTING_ID, None));
        sut.update_or_add(colored(DELETED_ID, None));
        sut
    }

    #[test]
    fn creation_event_is_emitted() {
        let mut sut = setup_existing();

        let mut changes = FullChanges::<DetailsEvent<Rc<TestEntry>>>::new();

        changes.append(sut.update_or_add(colored(NEW_ID, Red)));

        assert_eq!(
            sorted(changes.into()),
            vec![FullChange::new(
                Created(colored(NEW_ID, Red)),
                Deleted(colored_id(NEW_ID))
            ),]
        );
    }

    #[test]
    fn update_event_is_emitted() {
        let mut sut = setup_existing();

        let changes: Vec<_> = sut.update_or_add(colored(EXISTING_ID, Red)).into();

        assert_eq!(
            changes,
            vec![FullChange::new(
                Updated(colored(EXISTING_ID, Red)),
                Updated(colored(EXISTING_ID, None))
            )]
        );
    }

    #[test]
    fn delete_event_is_emitted() {
        let mut sut = setup_existing();

        let id = colored(EXISTING_ID, Red).get_id();
        let removed = sut.remove_by_id(&id);

        assert!(matches!(removed, Ok(_)));

        let changes: Vec<_> = removed.unwrap().into();

        assert_eq!(
            changes,
            vec![FullChange::new(
                Deleted(colored(EXISTING_ID, Red).get_id()),
                Created(colored(EXISTING_ID, None))
            )]
        );
    }

    #[test]
    #[rustfmt::skip]
    fn should_set_all_items() {
        let mut sut = setup_existing();

        let changes: Vec<_> = sut
            .set_all(vec![
                colored(ANY_NOT_USED_ENTRY_ID, None), // same
                colored(EXISTING_ID, Red),            // to update
                colored(NEW_ID, Red),                 // to create
                // colored(DELETED_ID, None),         // to delete
            ])
            .into();

        assert_eq!(
            changes,
            vec![
                FullChange::new(
                    Updated(colored(EXISTING_ID, Red)),
                    Updated(colored(EXISTING_ID, None))
                ),
                FullChange::new(Created(colored(NEW_ID, Red)), Deleted(colored_id(NEW_ID))),
                FullChange::new(
                    Deleted(colored_id(DELETED_ID)),
                    Created(colored(DELETED_ID, None))
                ),
            ]
        );
    }

    #[test]
    #[rustfmt::skip]
    fn should_set_some_items() {
        let mut sut = setup_existing();
        sut.add_new(colored(IGNORED_ID, None)).unwrap();

        let changes: Vec<_> = sut
            .set_some(
                |x| x.child_id != raw_colored_id(IGNORED_ID),
                vec![
                    colored(ANY_NOT_USED_ENTRY_ID, None), // same
                    colored(EXISTING_ID, Red),            // to update
                    colored(NEW_ID, Red),                 // to create
                    // colored(DELETED_ID, None),         // to delete
                ])
            .into();

        assert_eq!(
            changes,
            vec![
                FullChange::new(
                    Updated(colored(EXISTING_ID, Red)),
                    Updated(colored(EXISTING_ID, None))
                ),
                FullChange::new(Created(colored(NEW_ID, Red)), Deleted(colored_id(NEW_ID))),
                FullChange::new(
                    Deleted(colored_id(DELETED_ID)),
                    Created(colored(DELETED_ID, None))
                ),
            ]
        );
    }

    #[test]
    #[rustfmt::skip]
    fn should_ignore_not_changed_items_when_seting_some() {
        let mut sut = setup_existing();
        sut.add_new(colored(IGNORED_ID, None)).unwrap();

        let changes: Vec<_> = sut
            .set_some(
                |x| x.child_id != raw_colored_id(IGNORED_ID),
                vec![
                    colored(ANY_NOT_USED_ENTRY_ID, None),
                    colored(EXISTING_ID, None),
                    colored(DELETED_ID, None),
                ])
            .into();

        assert_eq!(changes, vec![]);
    }

    fn sorted<T>(mut changes: Vec<FullChange<DetailsEvent<T>>>) -> Vec<FullChange<DetailsEvent<T>>>
    where
        T: GetId,
        T::IdentifiableType: Owned,
        Id<T::IdentifiableType>: hash::Hash + Clone + Ord,
        Id<<T::IdentifiableType as Owned>::OwnerType>: Clone,
    {
        changes.sort_by_key(|c| DetailsEvent::get_id(c.redo()));
        changes
    }
}
