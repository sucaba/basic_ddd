use super::abstractions::*;
use crate::result::{AlreadyExists, NotFound};
use std::cmp::{Eq, PartialEq};
use std::fmt;
use std::hash;
use std::ops;
use std::result::Result as StdResult;
use std::slice;
use DbOwnedEvent::*;

pub enum DbOwnedEvent<T>
where
    T: GetId,
    T::IdentifiableType: Owned,
{
    Created(T),
    Updated(usize, T),
    Deleted(Id<<T as GetId>::IdentifiableType>),
    AllDeleted(Id<<T::IdentifiableType as Owned>::OwnerType>),
}

impl<T> DbOwnedEvent<T>
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
            AllDeleted(_) => None,
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

impl<T> fmt::Debug for DbOwnedEvent<T>
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
            AllDeleted(x) => write!(f, "DbOwnedEvent::AllDeleted({:?})", x),
        }
    }
}

impl<T> PartialEq for DbOwnedEvent<T>
where
    T: PartialEq + GetId,
    T::IdentifiableType: Owned,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Created(x), Created(y)) => x == y,
            (Updated(pos1, x), Updated(pos2, y)) => pos1 == pos2 && x == y,
            (Deleted(x), Deleted(y)) => x == y,
            (AllDeleted(x), AllDeleted(y)) => x == y,
            _ => false,
        }
    }
}

impl<T> Eq for DbOwnedEvent<T>
where
    T: Eq + GetId,
    T::IdentifiableType: Owned,
{
}

impl<T> Clone for DbOwnedEvent<T>
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
            AllDeleted(x) => AllDeleted(x.clone()),
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
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for OwnedCollection<T>
where
    T: GetId + Clone,
    T::IdentifiableType: Owned,
    DbOwnedEvent<T>: Clone,
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

impl<T> Streamable for OwnedCollection<T>
where
    T: GetId + Clone,
    T::IdentifiableType: Owned,
    Id<T::IdentifiableType>: hash::Hash + Clone,
{
    type EventType = DbOwnedEvent<T>;

    fn new_incomplete() -> Self {
        Self {
            inner: Vec::new(),
            complete: false,
        }
    }

    fn mark_complete(&mut self) {
        self.complete = true;
    }

    fn apply(&mut self, event: Self::EventType) {
        match event {
            Created(x) => self.inner.push(x),
            Updated(pos, x) => {
                self.inner[pos] = x;
            }
            Deleted(id) => {
                if let Some(i) = self.position_by_id(&id) {
                    self.inner.remove(i);
                }
            }
            AllDeleted(_) => self.inner.clear(),
        }
    }

    fn stream_to<S>(&mut self, _stream: &mut S)
    where
        S: StreamEvents<Self::EventType>,
    {
        todo!("remove")
    }
}

impl<T> OwnedCollection<T>
where
    T: GetId + Clone,
    T::IdentifiableType: Owned,
    Id<T::IdentifiableType>: hash::Hash + Clone,
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
            self.apply(Updated(pos, item.clone()));
            Ok(changes(Updated(pos, item)))
        } else {
            Err(NotFound(item))
        }
    }

    /**
     * Inserts a new item and returns `Ok(())` if item with the same id does not exist.
     * Returns `Err(item)` if item with the same already exists.
     */
    pub fn add_new(&mut self, item: T) -> StdResult<Changes<Self>, AlreadyExists<T>> {
        if let None = self.position_by_id(&item.get_id()) {
            self.apply(Created(item.clone()));
            Ok(changes(Created(item)))
        } else {
            Err(AlreadyExists(item))
        }
    }

    fn position_by_id(&self, id: &Id<T::IdentifiableType>) -> Option<usize> {
        self.inner.iter().position(|x| &x.get_id() == id)
    }

    pub fn remove_all(
        &mut self,
        owner_id: Id<<T::IdentifiableType as Owned>::OwnerType>,
    ) -> Changes<Self>
    where
        Id<<T::IdentifiableType as Owned>::OwnerType>: Clone,
    {
        self.apply(AllDeleted(owner_id.clone()));
        changes(AllDeleted(owner_id))
    }

    pub fn remove_by_id<'a>(
        &mut self,
        id: &'a Id<T::IdentifiableType>,
    ) -> StdResult<Changes<Self>, NotFound<&'a Id<T::IdentifiableType>>> {
        if let Some(_) = self.position_by_id(id) {
            self.apply(Deleted(id.clone()));
            Ok(changes(Deleted(id.clone())))
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

        changes.extend(sut.update_or_add(colored(1, Red)));
        changes.extend(sut.update_or_add(colored(2, Red)));

        assert_eq!(
            sorted(changes.into()),
            vec![Created(colored(1, Red)), Created(colored(2, Red))]
        );
    }

    #[test]
    fn update_event_is_streamed() {
        let mut sut = setup();

        let changes: Vec<_> = sut.update_or_add(colored(EXISTING_ID, Red)).into();

        assert_eq!(
            changes,
            vec![Updated(EXISTING_POS, colored(EXISTING_ID, Red))]
        );
    }

    #[test]
    fn delete_event_is_streamed() {
        let mut sut = setup();

        let id = colored(EXISTING_ID, Red).get_id();
        let removed = sut.remove_by_id(&id);

        assert!(matches!(removed, Ok(_)));

        let changes: Vec<_> = removed.unwrap().into();

        assert_eq!(changes, vec![Deleted(colored(EXISTING_ID, Red).get_id())]);
    }

    #[test]
    fn delete_all_event_is_streamed() {
        let mut sut = setup();

        let owner_id = (TestOwner { id: 1 }).get_id();

        let changes: Vec<_> = sut.remove_all(owner_id).into();

        assert_eq!(changes, vec![AllDeleted(owner_id)]);
    }

    fn sorted<T>(mut events: Vec<DbOwnedEvent<T>>) -> Vec<DbOwnedEvent<T>>
    where
        T: GetId,
        T::IdentifiableType: Owned,
        Id<T::IdentifiableType>: Clone + Ord,
    {
        events.sort_by_key(DbOwnedEvent::get_id);
        events
    }
}
