use super::abstractions::*;
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

impl<T> Eq for DbPrimaryEvent<T>
where
    T: GetId + Eq,
    Id<T::IdentifiableType>: Eq,
{
}

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

impl<T: GetId + Debug> Default for Primary<T> {
    fn default() -> Self {
        Self {
            inner: None,
            changes: Vec::new(),
        }
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

    pub fn set(&mut self, row: T)
    where
        T: Clone,
    {
        self.inner = Some(row.clone());
        self.changes.push(Updated(row));
    }

    pub fn update<F>(&mut self, f: F)
    where
        F: FnOnce(T) -> T,
        T: Clone,
    {
        self.inner = self.inner.take().map(f);
        self.changes
            .push(Updated(self.inner.clone().expect("Cannot update deleted")));
    }

    pub fn delete(&mut self) -> Option<T> {
        if let Some(created) = self.inner.as_ref() {
            self.changes.push(Deleted(created.get_id()));
            self.inner.take()
        } else {
            None
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
        });

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

        sut.delete();

        assert_eq!(sut.commit_changes(), vec![]);
    }

    #[test]
    fn should_combine_reduce_create_followed_by_update() {
        let mut sut = setup_new();

        for _ in 0..3 {
            sut.update(|mut x| {
                x.name = "ignored".into();
                x
            });
        }

        sut.update(|mut x| {
            x.name = "bar".into();
            x
        });

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
        });
        sut.update(|mut x| {
            x.name = "bar".into();
            x
        });

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
        });
        sut.delete();

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
            });
        }

        sut.update(|mut x| {
            x.name = "bar".into();
            x
        });

        assert_eq!(
            sut.commit_changes(),
            vec![Updated(MyEntity {
                id: 42,
                name: "bar".into(),
            })]
        );
    }
}
