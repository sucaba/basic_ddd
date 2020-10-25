use super::abstractions::*;
use crate::result::{NotFound, UpdateResult};
use std::cmp::{Eq, PartialEq};
use std::fmt::Debug;
use DbPrimaryEvent::*;

pub enum DbPrimaryEvent<T>
where
    T: GetId,
{
    Created(T),
    Updated(T),
    Deleted(Id<T::IdentifiableType>),
}

impl<T: GetId> DbPrimaryEvent<T> {
    fn merge(self, other: Self) -> Option<Self> {
        match (self, other) {
            (Created(_), Updated(m)) => Some(Created(m)),
            (Created(_), Deleted(_)) => None,
            (Updated(_), Updated(x)) => Some(Updated(x)),
            (Updated(_), Deleted(id)) => Some(Deleted(id)),
            _ => todo!("unsupported events to merge"),
        }
    }
}

impl<T> std::fmt::Debug for DbPrimaryEvent<T>
where
    T: Debug + GetId,
    Id<T::IdentifiableType>: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Created(x) => write!(f, "DbPrimaryEvent::Created({:?})", x),
            Updated(x) => write!(f, "DbPrimaryEvent::Updated({:?})", x),
            Deleted(x) => write!(f, "DbPrimaryEvent::Deleted({:?})", x),
        }
    }
}

impl<T> Clone for DbPrimaryEvent<T>
where
    T: Clone + GetId,
    Id<T::IdentifiableType>: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Created(x) => Created(x.clone()),
            Updated(x) => Updated(x.clone()),
            Deleted(x) => Deleted(x.clone()),
        }
    }
}

impl<T> Eq for DbPrimaryEvent<T> where T: GetId + Eq {}

impl<T> PartialEq for DbPrimaryEvent<T>
where
    T: GetId + PartialEq,
    Id<T::IdentifiableType>: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Created(x), Self::Created(y)) => x == y,
            (Self::Updated(x), Self::Updated(y)) => x == y,
            (Self::Deleted(x), Self::Deleted(y)) => x == y,
            _ => false,
        }
    }
}

pub struct Primary<T: GetId> {
    inner: Option<T>,
    changes: Vec<DbPrimaryEvent<T>>,
}

impl<T: GetId> Default for Primary<T> {
    fn default() -> Self {
        Self {
            inner: None,
            changes: Vec::new(),
        }
    }
}

impl<T> Clone for Primary<T>
where
    T: GetId + Clone,
    DbPrimaryEvent<T>: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            changes: self.changes.clone(),
        }
    }
}

impl<T: GetId + Eq> Eq for Primary<T> {}

impl<T: GetId + PartialEq> PartialEq for Primary<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner.eq(&other.inner) && self.changes.eq(&other.changes)
    }
}

impl<T> std::fmt::Debug for Primary<T>
where
    T: GetId + Debug,
    Vec<DbPrimaryEvent<T>>: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)?;
        self.changes.fmt(f)
    }
}

impl<T: GetId> Primary<T>
where
    DbPrimaryEvent<T>: Sized,
{
    pub fn new(row: T) -> Self
    where
        T: Clone,
    {
        Self {
            inner: Some(row.clone()),
            changes: vec![Created(row)],
        }
    }

    pub fn get(&self) -> &T {
        self.inner.as_ref().expect("not deleted")
    }

    pub fn set(&mut self, row: T) -> UpdateResult<T>
    where
        T: Clone,
    {
        if self.inner.is_some() {
            self.inner = Some(row.clone());
            self.changes.push(Updated(row));
            Ok(())
        } else {
            Err(NotFound(row))
        }
    }

    pub fn update<F>(&mut self, f: F) -> UpdateResult<()>
    where
        F: FnOnce(T) -> T,
        T: Clone,
    {
        if let Some(existing) = self.inner.take() {
            let modified = f(existing);
            self.inner = Some(modified.clone());
            self.changes.push(Updated(modified));
            Ok(())
        } else {
            Err(NotFound(()))
        }
    }

    pub fn delete(&mut self) -> UpdateResult<(), T> {
        if let Some(existing) = self.inner.take() {
            self.changes.push(Deleted(existing.get_id()));
            Ok(existing)
        } else {
            Err(NotFound(()))
        }
    }

    fn optimize(events: Vec<DbPrimaryEvent<T>>) -> Vec<DbPrimaryEvent<T>> {
        let mut iter = events.into_iter();
        if let Some(first) = iter.next() {
            iter.try_fold(first, DbPrimaryEvent::merge)
                .into_iter()
                .collect()
        } else {
            vec![]
        }
    }
}

impl<T> Streamable for Primary<T>
where
    T: GetId,
{
    type EventType = DbPrimaryEvent<T>;

    fn new_incomplete() -> Self {
        Self {
            inner: None,
            changes: Vec::new(),
        }
    }

    fn apply(&mut self, event: Self::EventType) {
        match event {
            Created(x) => self.inner = Some(x),
            Updated(x) => self.inner = Some(x),
            Deleted(_) => self.inner = None,
        }
    }

    fn stream_to<S>(&mut self, stream: &mut S)
    where
        S: StreamEvents<Self::EventType>,
    {
        let optimized = Self::optimize(std::mem::take(&mut self.changes));
        stream.stream(optimized);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use pretty_assertions::assert_eq;

    #[derive(Debug, Clone, Eq, PartialEq)]
    struct MyEntity {
        id: i32,
        name: String,
    }

    impl Identifiable for MyEntity {
        type IdType = i32;

        fn id(&self) -> Id<Self> {
            Id::new(self.id)
        }
    }

    const ID: i32 = 42;

    fn setup_new() -> Primary<MyEntity> {
        Primary::new(MyEntity {
            id: ID,
            name: "foo".into(),
        })
    }

    fn setup_saved() -> Primary<MyEntity> {
        let mut result = setup_new();
        result.assume_changes_saved();
        result
    }

    #[test]
    fn should_combine_create_update_into_create() {
        let mut sut = setup_new();

        sut.update(|mut x| {
            x.name = "bar".into();
            x
        })
        .unwrap();

        assert_eq!(
            sut.commit_changes(),
            vec![Created(MyEntity {
                id: 42,
                name: "bar".into(),
            })]
        );
    }

    #[test]
    fn should_annihilate_create_delete_changes() {
        let mut sut = setup_new();

        sut.delete().unwrap();

        assert_eq!(sut.commit_changes(), vec![]);
    }

    #[test]
    fn should_combine_reduce_create_followed_by_update() {
        let mut sut = setup_new();

        for _ in 0..3 {
            sut.update(|mut x| {
                x.name = "ignored".into();
                x
            })
            .unwrap();
        }

        sut.update(|mut x| {
            x.name = "bar".into();
            x
        })
        .unwrap();

        assert_eq!(
            sut.commit_changes(),
            vec![Created(MyEntity {
                id: 42,
                name: "bar".into(),
            })]
        );
    }

    #[test]
    fn should_combine_update_update() {
        let mut sut = setup_saved();

        sut.update(|mut x| {
            x.name = "ignored".into();
            x
        })
        .unwrap();
        sut.update(|mut x| {
            x.name = "bar".into();
            x
        })
        .unwrap();

        assert_eq!(
            sut.commit_changes(),
            vec![Updated(MyEntity {
                id: 42,
                name: "bar".into(),
            })]
        );
    }

    #[test]
    fn should_combine_update_delete() {
        let mut sut = setup_saved();

        sut.update(|mut x| {
            x.name = "ignored".into();
            x
        })
        .unwrap();
        sut.delete().unwrap();

        assert_eq!(
            sut.commit_changes(),
            vec![Deleted(
                MyEntity {
                    id: 42,
                    name: "bar".into(),
                }
                .id()
            )]
        );
    }

    #[test]
    fn should_combine_multiple_updates() {
        let mut sut = setup_saved();

        for _ in 0..3 {
            sut.update(|mut x| {
                x.name = "ignored".into();
                x
            })
            .unwrap();
        }

        sut.update(|mut x| {
            x.name = "bar".into();
            x
        })
        .unwrap();

        assert_eq!(
            sut.commit_changes(),
            vec![Updated(MyEntity {
                id: 42,
                name: "bar".into(),
            })]
        );
    }
}
