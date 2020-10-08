use super::abstractions::*;
use std::cmp::{Eq, PartialEq};
use std::fmt::Debug;

pub enum DbPrimaryEvent<T>
where
    T: HasId,
{
    Created(T),
    Updated(T),
    Deleted(Id<T>),
}

impl<T> std::fmt::Debug for DbPrimaryEvent<T>
where
    T: Debug + HasId,
    Id<T>: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbPrimaryEvent::Created(x) => write!(f, "DbPrimaryEvent::Created({:?})", x),
            DbPrimaryEvent::Updated(x) => write!(f, "DbPrimaryEvent::Updated({:?})", x),
            DbPrimaryEvent::Deleted(x) => write!(f, "DbPrimaryEvent::Deleted({:?})", x),
        }
    }
}

impl<T> Eq for DbPrimaryEvent<T>
where
    T: HasId + Eq,
    Id<T>: Eq,
{
}

impl<T> PartialEq for DbPrimaryEvent<T>
where
    T: HasId + PartialEq,
    Id<T>: PartialEq,
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

pub struct Primary<T: HasId> {
    inner: Option<T>,
    changes: Vec<DbPrimaryEvent<T>>,
}

impl<T: HasId + Debug> Default for Primary<T> {
    fn default() -> Self {
        Self {
            inner: None,
            changes: Vec::new(),
        }
    }
}

impl<T> std::fmt::Debug for Primary<T>
where
    T: HasId + Debug,
    Vec<DbPrimaryEvent<T>>: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)?;
        self.changes.fmt(f)
    }
}

impl<T: HasId> Primary<T>
where
    DbPrimaryEvent<T>: Sized,
{
    pub fn new(row: T) -> Self
    where
        T: Clone,
    {
        Self {
            inner: Some(row.clone()),
            changes: vec![DbPrimaryEvent::Created(row)],
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
        self.changes.push(DbPrimaryEvent::Updated(row));
    }

    pub fn update<F>(&mut self, f: F)
    where
        F: FnOnce(T) -> T,
        T: Clone,
    {
        self.inner = self.inner.take().map(f);
        self.changes.push(DbPrimaryEvent::Updated(
            self.inner.clone().expect("Cannot update deleted"),
        ));
    }

    pub fn delete(&mut self, id: Id<T>) {
        self.inner = None;
        self.changes.push(DbPrimaryEvent::Deleted(id));
    }

    fn optimize(mut events: Vec<DbPrimaryEvent<T>>) -> Vec<DbPrimaryEvent<T>>
    where
        T: Default,
    {
        use std::mem::take;
        use DbPrimaryEvent::*;

        match events.as_mut_slice() {
            [Created(_), Updated(m), ..] => {
                events[1] = Created(take(m));
                events.remove(0);
                events
            }
            [Updated(_), Updated(_), ..] => {
                events.remove(0);
                events
            }
            _ => events,
        }
    }
}

impl<T> Streamable for Primary<T>
where
    T: HasId + Default,
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
    use pretty_assertions::assert_eq;

    #[derive(Debug, Clone, Eq, PartialEq, Default)]
    struct MyEntity {
        id: i32,
        name: String,
    }

    impl HasId for MyEntity {
        type IdType = i32;

        fn id(&self) -> Id<Self> {
            Id::new(self.id)
        }
    }

    #[test]
    fn should_combine_create_update_into_create() {
        let mut sut = Primary::new(MyEntity {
            id: 42,
            name: "foo".into(),
        });
        sut.update(|mut x| {
            x.name = "bar".into();
            x
        });

        assert_eq!(
            sut.commit_changes(),
            vec![DbPrimaryEvent::Created(MyEntity {
                id: 42,
                name: "bar".into(),
            })]
        );
    }

    #[test]
    fn should_combine_update_update() {
        let mut sut = Primary::new(MyEntity {
            id: 42,
            name: "foo".into(),
        });
        let _ = sut.commit_changes();

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
            vec![DbPrimaryEvent::Updated(MyEntity {
                id: 42,
                name: "bar".into(),
            })]
        );
    }
}
