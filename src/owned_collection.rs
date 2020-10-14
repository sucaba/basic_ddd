use super::abstractions::*;
use std::cmp::{Eq, PartialEq};
use std::fmt::Debug;
use std::hash::Hash;
use DbOwnedEvent::*;

pub enum DbOwnedEvent<T>
where
    T: HasId + HasOwner,
{
    Created(T),
    Updated(T),
    Deleted(Id<T>),
    AllDeleted(Id<T::OwnerType>),
}

impl<T: HasId + HasOwner> DbOwnedEvent<T>
where
    Id<T>: Clone,
{
    fn id(&self) -> Option<Id<T>> {
        match self {
            Created(x) => Some(x.id()),
            Updated(x) => Some(x.id()),
            Deleted(id) => Some(id.clone()),
            AllDeleted(_) => None,
        }
    }

    fn merge(&mut self, new: Self) -> EventMergeResult {
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

impl<T> std::fmt::Debug for DbOwnedEvent<T>
where
    T: Debug + HasId + HasOwner,
    Id<T>: Debug,
    Id<T::OwnerType>: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Created(x) => write!(f, "DbOwnedEvent::Created({:?})", x),
            Updated(x) => write!(f, "DbOwnedEvent::Updated({:?})", x),
            Deleted(x) => write!(f, "DbOwnedEvent::Deleted({:?})", x),
            AllDeleted(x) => write!(f, "DbOwnedEvent::AllDeleted({:?})", x),
        }
    }
}

impl<T> PartialEq for DbOwnedEvent<T>
where
    T: PartialEq + HasId + HasOwner,
    Id<T>: PartialEq,
    Id<T::OwnerType>: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Created(x), Created(y)) => x == y,
            (Updated(x), Updated(y)) => x == y,
            (Deleted(x), Deleted(y)) => x == y,
            (AllDeleted(x), AllDeleted(y)) => x == y,
            _ => false,
        }
    }
}

impl<T> Eq for DbOwnedEvent<T>
where
    T: Eq + HasId + HasOwner,
    Id<T>: Eq,
    Id<T::OwnerType>: Eq,
{
}

pub struct OwnedCollection<T: HasId + HasOwner> {
    inner: Vec<T>,
    changes: Vec<DbOwnedEvent<T>>,
}

impl<T: HasId + HasOwner + Debug> Debug for OwnedCollection<T>
where
    Id<T>: Debug,
    Id<T::OwnerType>: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        Debug::fmt(&self.inner, f)?;
        f.write_str("\nchanges:\n")?;
        Debug::fmt(&self.changes, f)
    }
}

impl<T, I> std::ops::Index<I> for OwnedCollection<T>
where
    T: HasId + HasOwner,
    I: std::slice::SliceIndex<[T]>,
{
    type Output = I::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        &self.inner[index]
    }
}

impl<'a, T> IntoIterator for &'a OwnedCollection<T>
where
    T: HasId + HasOwner,
{
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

impl<T> Default for OwnedCollection<T>
where
    T: HasId + HasOwner,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Extend<T> for OwnedCollection<T>
where
    T: HasId + HasOwner + Clone,
    Id<T>: Eq,
{
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for item in iter.into_iter() {
            self.update_or_add(item);
        }
    }
}

impl<T> Streamable for OwnedCollection<T>
where
    T: HasId + HasOwner,
    Id<T>: Eq + Hash + Clone,
{
    type EventType = DbOwnedEvent<T>;

    fn stream_to<S>(&mut self, stream: &mut S)
    where
        S: StreamEvents<Self::EventType>,
    {
        let optimized = Self::optimize(std::mem::take(&mut self.changes));
        stream.stream(optimized);
    }
}

impl<T> OwnedCollection<T>
where
    T: HasId + HasOwner,
{
    pub fn new() -> Self {
        Self {
            inner: Vec::new(),
            changes: Vec::new(),
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.inner.iter()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn update_or_add(&mut self, item: T)
    where
        T: Clone,
        Id<T>: Eq,
    {
        let _ = self.update(item).or_else(|x| self.add_new(x));
    }

    /**
     * Updates existing item or returns item back as a Result::Err
     */
    pub fn update(&mut self, item: T) -> std::result::Result<(), T>
    where
        T: Clone,
        Id<T>: Eq,
    {
        if let Some(pos) = self.position_by_id(&item.id()) {
            self.inner[pos] = item.clone();
            self.changes.push(Updated(item));
            Ok(())
        } else {
            Err(item)
        }
    }

    /**
     * Inserts a new item and returns `Ok(())` if item with the same id does not exist.
     * Returns `Err(item)` if item with the same already exists.
     */
    pub fn add_new(&mut self, item: T) -> std::result::Result<(), T>
    where
        T: Clone,
        Id<T>: Eq,
    {
        if let None = self.position_by_id(&item.id()) {
            self.inner.push(item.clone());
            self.changes.push(Created(item));
            Ok(())
        } else {
            Err(item)
        }
    }

    fn position_by_id(&self, id: &Id<T>) -> Option<usize>
    where
        Id<T>: Eq,
    {
        self.inner.iter().position(|x| &x.id() == id)
    }

    pub fn remove_all(&mut self, owner_id: Id<T::OwnerType>) {
        self.inner.clear();
        self.changes.push(AllDeleted(owner_id));
    }

    pub fn remove_by_id(&mut self, id: &Id<T>) -> bool
    where
        Id<T>: Eq + Clone,
    {
        if let Some(i) = self.position_by_id(id) {
            self.inner.remove(i);
            self.changes.push(Deleted(id.clone()));
            true
        } else {
            false
        }
    }

    fn optimize(events: Vec<DbOwnedEvent<T>>) -> Vec<DbOwnedEvent<T>>
    where
        Id<T>: Eq + Hash + Clone,
    {
        use std::collections::hash_map::Entry::*;
        use std::collections::HashMap;
        use EventMergeResult::Annihilated;

        let mut event_per_id = HashMap::new();

        for e in events {
            if let Some(id) = e.id() {
                match event_per_id.entry(id) {
                    Vacant(v) => {
                        v.insert(e);
                    }
                    Occupied(mut o) => {
                        if let Annihilated = o.get_mut().merge(e) {
                            o.remove();
                        }
                    }
                }
            } else {
                return vec![e];
            }
        }

        event_per_id.into_iter().map(|(_, v)| v).collect()
    }
}

#[cfg(test)]
mod owned_collection_tests {

    use super::*;
    use crate::test_utils::*;
    use pretty_assertions::assert_eq;
    use std::cmp::{Eq, PartialEq};
    use Color::*;

    struct TestOwner {
        id: i32,
    }

    impl HasId for TestOwner {
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

    impl HasId for TestEntry {
        type IdType = String;

        fn id(&self) -> Id<Self> {
            Id::new(self.child_id.clone())
        }
    }

    impl HasOwner for TestEntry {
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

    fn colored(seed: usize, c: Color) -> TestEntry {
        TestEntry {
            owner_id: 1,
            child_id: (100 + seed).to_string(),
            name: format!("{:#?}", c),
        }
    }

    fn setup_saved() -> OwnedCollection<TestEntry> {
        let mut sut = OwnedCollection::new();
        sut.update_or_add(colored(EXISTING_ID, None));
        sut.update_or_add(colored(ANY_NOT_USED_ENTRY_ID, None));
        sut.assume_changes_saved();
        sut
    }

    fn setup_new() -> OwnedCollection<TestEntry> {
        OwnedCollection::new()
    }

    #[test]
    fn creation_event_is_streamed() {
        let mut sut = setup_saved();

        sut.update_or_add(colored(1, Red));
        sut.update_or_add(colored(2, Red));

        assert_eq!(
            sorted(sut.commit_changes()),
            vec![Created(colored(1, Red)), Created(colored(2, Red))]
        );
    }

    #[test]
    fn update_event_is_streamed() {
        let mut sut = setup_saved();

        sut.update_or_add(colored(EXISTING_ID, Red));

        assert_eq!(
            sut.commit_changes(),
            vec![Updated(colored(EXISTING_ID, Red))]
        );
    }

    #[test]
    fn delete_event_is_streamed() {
        let mut sut = setup_saved();

        let removed = sut.remove_by_id(&colored(EXISTING_ID, Red).id());

        assert!(removed);

        assert_eq!(
            sut.commit_changes(),
            vec![Deleted(colored(EXISTING_ID, Red).id())]
        );
    }

    #[test]
    fn delete_all_event_is_streamed() {
        let mut sut = setup_saved();

        let owner_id = (TestOwner { id: 1 }).id();

        sut.remove_all(owner_id);

        assert_eq!(sut.commit_changes(), vec![AllDeleted(owner_id)]);
    }

    #[test]
    fn should_optimize_create_update_of_single_entry() {
        let mut sut = setup_new();

        sut.update_or_add(colored(1, Red));
        sut.update_or_add(colored(1, Blue));

        assert_eq!(sut.commit_changes(), vec![Created(colored(1, Blue))]);
    }

    #[test]
    fn should_optimize_create_update_of_multiple_entries() {
        let mut sut = setup_new();

        sut.update_or_add(colored(1, Red));
        sut.update_or_add(colored(1, Blue));

        sut.update_or_add(colored(2, Red));
        sut.update_or_add(colored(2, Blue));

        assert_eq!(
            sorted(sut.commit_changes()),
            vec![Created(colored(1, Blue)), Created(colored(2, Blue)),]
        );
    }

    #[test]
    fn should_optimize_update_delete() {
        let mut sut = setup_saved();

        sut.update_or_add(colored(EXISTING_ID, Red));

        let id = colored(EXISTING_ID, Red).id();
        sut.remove_by_id(&id);

        assert_eq!(sorted(sut.commit_changes()), vec![Deleted(id)]);
    }

    #[test]
    fn should_optimize_create_delete_by_annihilation() {
        let mut sut = setup_new();

        sut.update_or_add(colored(1, Red));
        sut.remove_by_id(&colored(1, Blue).id());

        assert_eq!(sorted(sut.commit_changes()), vec![]);
    }

    #[test]
    fn should_optimize_muttiple_create_delete_on_the_same_entry_by_annihilation() {
        let mut sut = setup_new();

        for _ in 0..3 {
            sut.update_or_add(colored(1, Red));
            sut.remove_by_id(&colored(1, Blue).id());
        }

        assert_eq!(sorted(sut.commit_changes()), vec![]);
    }

    #[test]
    fn should_optimize_create_delete_by_annihilation_independently_of_other_entries() {
        let mut sut = setup_new();

        sut.update_or_add(colored(1, Red));
        sut.remove_by_id(&colored(1, Blue).id());

        sut.update_or_add(colored(2, Red));

        assert_eq!(sorted(sut.commit_changes()), vec![Created(colored(2, Red))]);
    }

    #[test]
    fn should_optimize_multiple_create_delete_by_annihilation_independently_of_other_entries() {
        let mut sut = setup_new();

        sut.update_or_add(colored(1, Red));
        sut.remove_by_id(&colored(1, Blue).id());

        sut.update_or_add(colored(2, Red));
        sut.remove_by_id(&colored(2, Blue).id());

        sut.update_or_add(colored(3, Red));

        assert_eq!(sorted(sut.commit_changes()), vec![Created(colored(3, Red))]);
    }

    fn sorted(mut events: Vec<DbOwnedEvent<TestEntry>>) -> Vec<DbOwnedEvent<TestEntry>> {
        events.sort_by_key(DbOwnedEvent::id);
        events
    }
}
