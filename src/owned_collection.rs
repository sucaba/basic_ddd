// TODO: Remove std::hash references
use super::abstractions::*;
use crate::result::{AlreadyExists, NotFound};
use std::cmp::{Eq, PartialEq};
use std::fmt;
use std::hash;
use std::mem;
use std::ops;
use std::result::Result as StdResult;
use std::slice;
use OwnedEvent::*;

pub enum OwnedEvent<T>
where
    T: GetId,
    T::IdentifiableType: Owned,
{
    Created(T),
    Updated(usize, T),
    Deleted(Id<<T as GetId>::IdentifiableType>),
}

impl<T> OwnedEvent<T>
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

impl<T> fmt::Debug for OwnedEvent<T>
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

impl<T> PartialEq for OwnedEvent<T>
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

impl<T> Eq for OwnedEvent<T>
where
    T: Eq + GetId,
    T::IdentifiableType: Owned,
{
}

impl<T> Clone for OwnedEvent<T>
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

pub struct OwnedCollection<T>
where
    T: GetId,
    T::IdentifiableType: Owned,
{
    inner: Vec<T>,
    complete: bool,
}

impl<T> Eq for OwnedCollection<T>
where
    T: GetId + Eq,
    T::IdentifiableType: Owned,
{
}

impl<T> PartialEq for OwnedCollection<T>
where
    T: GetId + PartialEq,
    T::IdentifiableType: Owned,
{
    fn eq(&self, other: &Self) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl<T: GetId + fmt::Debug> fmt::Debug for OwnedCollection<T>
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

impl<T, I> ops::Index<I> for OwnedCollection<T>
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

impl<'a, T> IntoIterator for &'a OwnedCollection<T>
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

impl<T> Default for OwnedCollection<T>
where
    T: GetId + Clone,
    T::IdentifiableType: Owned,
    Id<T::IdentifiableType>: hash::Hash + Clone,
    Id<<T::IdentifiableType as Owned>::OwnerType>: Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for OwnedCollection<T>
where
    T: GetId + Clone,
    T::IdentifiableType: Owned,
    OwnedEvent<T>: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            complete: self.complete,
        }
    }
}

/// Extend trait cannot be implemented because \
/// we cannot return Changes out of extend method:
///
/// impl<T> Extend<T> for OwnedCollection<T>
/// where
///     T: GetId + Clone,
///     T::IdentifiableType: Owned,
///     Id<T::IdentifiableType>: hash::Hash + Clone,
/// {
///     fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) -> Changes<<Self as Streamable>::EventType> {
///         iter.into_iter()
///             .map(|item| self.update_or_add(item))
///             .flatten()
///             .collect()
///     }
/// }
///

impl<T> Changable for OwnedCollection<T>
where
    T: GetId + Clone,
    T::IdentifiableType: Owned,
    Id<T::IdentifiableType>: hash::Hash + Clone,
    Id<<T::IdentifiableType as Owned>::OwnerType>: Clone,
{
    type EventType = OwnedEvent<T>;
    type ChangeUnit = BasicChange<Self>;

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

impl<T> OwnedCollection<T>
where
    T: GetId + Clone,
    T::IdentifiableType: Owned,
    Id<T::IdentifiableType>: hash::Hash + Clone,
    Id<<T::IdentifiableType as Owned>::OwnerType>: Clone,
{
    pub fn new() -> Self {
        Self {
            inner: Vec::new(),
            complete: true,
        }
    }

    pub fn iter(&self) -> slice::Iter<'_, T> {
        self.inner.iter()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn update_or_add(&mut self, item: T) -> Changes<Self>
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
    pub fn update(&mut self, item: T) -> StdResult<Changes<Self>, NotFound<T>> {
        if let Some(pos) = self.position_by_id(&item.get_id()) {
            Ok(Changes::from_application(Updated(pos, item), self))
        } else {
            Err(NotFound(item))
        }
    }

    /**
     * Inserts a new item and returns `Ok(())` if item with the same id does not exist.
     * Returns `Err(item)` if item with the same already exists.
     */
    pub fn add_new(&mut self, item: T) -> StdResult<Changes<Self>, AlreadyExists<T>> {
        let id = item.get_id();
        if let None = self.position_by_id(&id) {
            Ok(Changes::from_application(Created(item), self))
        } else {
            Err(AlreadyExists(item))
        }
    }

    fn position_by_id(&self, id: &Id<T::IdentifiableType>) -> Option<usize> {
        self.inner.iter().position(|x| &x.get_id() == id)
    }

    pub fn remove(
        &mut self,
        item: &T,
    ) -> StdResult<Changes<Self>, NotFound<Id<T::IdentifiableType>>> {
        let id = item.get_id();
        match self.remove_by_id(&id) {
            Err(_) => Err(NotFound(id)),
            Ok(changes) => Ok(changes),
        }
    }

    pub fn remove_by_id<'a>(
        &mut self,
        id: &'a Id<T::IdentifiableType>,
    ) -> StdResult<Changes<Self>, NotFound<&'a Id<T::IdentifiableType>>> {
        if let Some(_) = self.position_by_id(id) {
            Ok(Changes::from_application(Deleted(id.clone()), self))
        } else {
            Err(NotFound(id))
        }
    }
}

#[cfg(test)]
mod owned_collection_tests {

    use super::*;
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

    #[derive(Debug, Eq, PartialEq, Clone)]
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

    type Sut = OwnedCollection<Rc<TestEntry>>;

    fn setup() -> Sut {
        let mut sut = OwnedCollection::new();
        sut.update_or_add(colored(ANY_NOT_USED_ENTRY_ID, None));
        sut.update_or_add(colored(EXISTING_ID, None));
        sut
    }

    #[test]
    fn creation_event_is_streamed() {
        let mut sut = setup();

        let mut changes = Changes::<Sut>::new();

        changes.extend_changes(sut.update_or_add(colored(1, Red)));
        changes.extend_changes(sut.update_or_add(colored(2, Red)));

        assert_eq!(
            sorted(changes.into()),
            vec![
                BasicChange {
                    redo: Created(colored(1, Red)),
                    undo: Deleted(colored(1, Red).get_id())
                },
                BasicChange {
                    redo: Created(colored(2, Red)),
                    undo: Deleted(colored(2, Red).get_id())
                }
            ]
        );
    }

    #[test]
    fn update_event_is_streamed() {
        let mut sut = setup();

        let changes: Vec<_> = sut.update_or_add(colored(EXISTING_ID, Red)).into();

        assert_eq!(
            changes,
            vec![BasicChange {
                redo: Updated(EXISTING_POS, colored(EXISTING_ID, Red)),
                undo: Updated(EXISTING_POS, colored(EXISTING_ID, None))
            }]
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
            vec![BasicChange {
                redo: Deleted(colored(EXISTING_ID, Red).get_id()),
                undo: Created(colored(EXISTING_ID, None))
            }]
        );
    }

    fn sorted<T>(
        mut events: Vec<BasicChange<OwnedCollection<T>>>,
    ) -> Vec<BasicChange<OwnedCollection<T>>>
    where
        T: GetId + Clone,
        T::IdentifiableType: Owned,
        Id<T::IdentifiableType>: hash::Hash + Clone + Ord,
        Id<<T::IdentifiableType as Owned>::OwnerType>: Clone,
    {
        events.sort_by_key(|BasicChange { redo, .. }| OwnedEvent::get_id(&redo));
        events
    }
}
