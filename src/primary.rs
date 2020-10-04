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

pub struct Primary<T: HasId + Debug> {
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

impl<T: HasId + Debug> Primary<T> {
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

    pub fn update(&mut self, row: T)
    where
        T: Clone,
    {
        self.inner = Some(row.clone());
        self.changes.push(DbPrimaryEvent::Updated(row));
    }

    pub fn delete(&mut self, id: Id<T>) {
        self.inner = None;
        self.changes.push(DbPrimaryEvent::Deleted(id));
    }
}

impl<T> Streamable for Primary<T>
where
    T: HasId + Debug,
{
    type EventType = DbPrimaryEvent<T>;

    fn stream_to<S>(&mut self, stream: &mut S)
    where
        S: StreamEvents<Self::EventType>,
    {
        stream.stream(std::mem::replace(&mut self.changes, Vec::new()));
    }
}
