use super::identifiable::*;
use crate::changable::Changable;
use crate::change_abs::AppliedChange;
use crate::changes::FullChanges;
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
    Updated(usize, T),
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
            Updated(_, x) => Some(x.get_id()),
            Deleted(id) => Some(id.clone()),
        }
    }

    pub fn merge(&mut self, new: Self) -> EventMergeResult {
        use EventMergeResult::*;

        match (self as &_, new) {
            (Created(_), Updated(_, now)) => {
                *self = Created(now);
                Combined
            }
            (Updated(_, _), Updated(pos, now)) => {
                *self = Updated(pos, now);
                Combined
            }
            (Created(_), Deleted(_)) => Annihilated,
            (Updated(_, _), Deleted(id)) => {
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
            Updated(pos, x) => write!(f, "DbOwnedEvent::Updated({:?}, {:?})", pos, x),
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
            (Updated(pos1, x), Updated(pos2, y)) => pos1 == pos2 && x == y,
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
            Updated(pos, x) => Updated(pos.clone(), x.clone()),
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
    T: GetId + fmt::Debug,
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

impl<T, C> Changable for Details<T, C>
where
    T: GetId,
    T::IdentifiableType: Owned,
    Id<T::IdentifiableType>: hash::Hash + Clone,
    Id<<T::IdentifiableType as Owned>::OwnerType>: Clone,
{
    type EventType = DetailsEvent<T>;

    fn apply(&mut self, event: Self::EventType) -> Self::EventType {
        match event {
            Created(x) => {
                let id = x.get_id();
                self.inner.push(x);
                Deleted(id)
            }
            Updated(pos, x) => {
                let old = mem::replace(&mut self.inner[pos], x);
                Updated(pos, old)
            }
            Deleted(id) => {
                let i = self.position_by_id(&id).expect("Dev error: id not found");
                let old = self.inner.remove(i);
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

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn update_or_add(&mut self, item: T) -> C
    where
        T: fmt::Debug,
    {
        self.update(item)
            .or_else(|NotFound(x)| self.add_new(x))
            .unwrap()
    }

    /**
     * Updates existing item or returns item back as a Result::Err
     */
    pub fn update(&mut self, item: T) -> StdResult<C, NotFound<T>> {
        if let Some(pos) = self.position_by_id(&item.get_id()) {
            Ok(self.applied(Updated(pos, item)))
        } else {
            Err(NotFound(item))
        }
    }

    /**
     * Inserts a new item and returns `Ok(())` if item with the same id does not exist.
     * Returns `Err(item)` if item with the same already exists.
     */
    pub fn add_new(&mut self, item: T) -> StdResult<C, AlreadyExists<T>> {
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
    const EXISTING_POS: usize = 1;

    fn colored(seed: usize, c: Color) -> Rc<TestEntry> {
        TestEntry {
            owner_id: 1,
            child_id: (100 + seed).to_string(),
            name: format!("{:#?}", c),
        }
        .into()
    }

    type Sut = Details<Rc<TestEntry>>;

    fn setup() -> Sut {
        let mut sut = Details::new();
        sut.update_or_add(colored(ANY_NOT_USED_ENTRY_ID, None));
        sut.update_or_add(colored(EXISTING_ID, None));
        sut
    }

    #[test]
    fn creation_event_is_streamed() {
        let mut sut = setup();

        let mut changes = FullChanges::<DetailsEvent<Rc<TestEntry>>>::new();

        changes.append(sut.update_or_add(colored(1, Red)));
        changes.append(sut.update_or_add(colored(2, Red)));

        assert_eq!(
            sorted(changes.into()),
            vec![
                FullChange::new(Created(colored(1, Red)), Deleted(colored(1, Red).get_id())),
                FullChange::new(Created(colored(2, Red)), Deleted(colored(2, Red).get_id()))
            ]
        );
    }

    #[test]
    fn update_event_is_streamed() {
        let mut sut = setup();

        let changes: Vec<_> = sut.update_or_add(colored(EXISTING_ID, Red)).into();

        assert_eq!(
            changes,
            vec![FullChange::new(
                Updated(EXISTING_POS, colored(EXISTING_ID, Red)),
                Updated(EXISTING_POS, colored(EXISTING_ID, None))
            )]
        );
    }

    #[test]
    fn delete_event_is_streamed() {
        let mut sut = setup();

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

    fn sorted<T>(mut events: Vec<FullChange<DetailsEvent<T>>>) -> Vec<FullChange<DetailsEvent<T>>>
    where
        T: GetId,
        T::IdentifiableType: Owned,
        Id<T::IdentifiableType>: hash::Hash + Clone + Ord,
        Id<<T::IdentifiableType as Owned>::OwnerType>: Clone,
    {
        events.sort_by_key(|c| DetailsEvent::get_id(c.redo()));
        events
    }
}
