use super::abstractions::*;
use std::cmp::{Eq, PartialEq};
use std::fmt::Debug;

pub enum DbOwnedEvent<T>
where
    T: HasId + HasOwner,
{
    Created(T),
    Updated(T),
    Deleted(Id<T>),
    AllDeleted(Id<T::OwnerType>),
}

impl<T> std::fmt::Debug for DbOwnedEvent<T>
where
    T: Debug + HasId + HasOwner,
    Id<T>: Debug,
    Id<T::OwnerType>: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbOwnedEvent::Created(x) => write!(f, "DbOwnedEvent::Created({:?})", x),
            DbOwnedEvent::Updated(x) => write!(f, "DbOwnedEvent::Updated({:?})", x),
            DbOwnedEvent::Deleted(x) => write!(f, "DbOwnedEvent::Deleted({:?})", x),
            DbOwnedEvent::AllDeleted(x) => write!(f, "DbOwnedEvent::AllDeleted({:?})", x),
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
            (Self::Created(x), Self::Created(y)) => x == y,
            (Self::Updated(x), Self::Updated(y)) => x == y,
            (Self::Deleted(x), Self::Deleted(y)) => x == y,
            (Self::AllDeleted(x), Self::AllDeleted(y)) => x == y,
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

impl<T> Streamable for OwnedCollection<T>
where
    T: HasId + HasOwner,
{
    type EventType = DbOwnedEvent<T>;

    fn stream_to<S>(&mut self, stream: &mut S)
    where
        S: StreamEvents<Self::EventType>,
    {
        stream.stream(std::mem::replace(&mut self.changes, Vec::new()));
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

    pub fn add(&mut self, item: T)
    where
        T: Clone,
        Id<T>: Eq,
    {
        if let Some(pos) = self.position_by_id(&item.id()) {
            self.inner[pos] = item.clone();
            self.changes.push(DbOwnedEvent::Updated(item));
        } else {
            self.inner.push(item.clone());
            self.changes.push(DbOwnedEvent::Created(item));
        }
    }

    fn position_by_id(&self, id: &Id<T>) -> Option<usize>
    where
        Id<T>: Eq,
    {
        self.inner.iter().position(|x| &x.id() == id)
    }

    pub fn remove_by_id(&mut self, id: &Id<T>) -> bool
    where
        Id<T>: Eq + Clone,
    {
        if let Some(i) = self.position_by_id(id) {
            let _ = self.inner.remove(i);
            self.changes.push(DbOwnedEvent::Deleted(id.clone()));
            true
        } else {
            false
        }
    }
}

impl<T> Extend<T> for OwnedCollection<T>
where
    T: HasId + HasOwner + Clone,
    Id<T>: Eq,
{
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for item in iter.into_iter() {
            self.add(item);
        }
    }
}

#[cfg(test)]
mod owned_collection_tests {

    use super::*;
    use pretty_assertions::assert_eq;
    use std::cmp::{Eq, PartialEq};

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

    #[test]
    fn creation_event_is_streamed() {
        let mut sut = OwnedCollection::new();

        let entry1 = TestEntry {
            owner_id: 1,
            child_id: "101".into(),
            name: "red".into(),
        };

        let entry2 = TestEntry {
            owner_id: 1,
            child_id: "102".into(),
            name: "red".into(),
        };

        sut.add(entry1.clone());
        sut.add(entry2.clone());

        assert_eq!(
            sut.commit_changes(),
            vec![DbOwnedEvent::Created(entry1), DbOwnedEvent::Created(entry2)]
        );
    }

    #[test]
    fn update_event_is_streamed() {
        let mut sut = OwnedCollection::new();

        let entry1 = TestEntry {
            owner_id: 1,
            child_id: "101".into(),
            name: "red".into(),
        };

        let entry2 = TestEntry {
            owner_id: 1,
            child_id: "101".into(),
            name: "green".into(),
        };

        sut.add(entry1.clone());

        sut.add(entry2.clone());

        assert_eq!(
            sut.commit_changes(),
            vec![DbOwnedEvent::Created(entry1), DbOwnedEvent::Updated(entry2)]
        );
    }

    #[test]
    fn delete_event_is_streamed() {
        let mut sut = OwnedCollection::new();

        let entry1 = TestEntry {
            owner_id: 1,
            child_id: "101".into(),
            name: "red".into(),
        };

        let entry2 = TestEntry {
            owner_id: 1,
            child_id: "102".into(),
            name: "green".into(),
        };

        sut.add(entry1.clone());
        sut.add(entry2.clone());

        sut.remove_by_id(&entry1.id());

        assert_eq!(
            sut.commit_changes(),
            vec![
                DbOwnedEvent::Created(entry1.clone()),
                DbOwnedEvent::Created(entry2),
                DbOwnedEvent::Deleted(entry1.id())
            ]
        );
    }
}
