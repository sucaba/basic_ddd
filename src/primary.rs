use super::abstractions::*;
use crate::result::NotFound;
use std::cmp::{Eq, PartialEq};
use std::fmt;
use std::result::Result as StdResult;
use PrimaryEvent::*;

pub enum PrimaryEvent<T>
where
    T: GetId,
{
    Created(T),
    Updated(T),
    Deleted(Id<T::IdentifiableType>),
}

impl<T> fmt::Debug for PrimaryEvent<T>
where
    T: fmt::Debug + GetId,
    Id<T::IdentifiableType>: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Created(x) => write!(f, "DbPrimaryEvent::Created({:?})", x),
            Updated(x) => write!(f, "DbPrimaryEvent::Updated({:?})", x),
            Deleted(x) => write!(f, "DbPrimaryEvent::Deleted({:?})", x),
        }
    }
}

impl<T> Clone for PrimaryEvent<T>
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

impl<T> Eq for PrimaryEvent<T> where T: GetId + Eq {}

impl<T> PartialEq for PrimaryEvent<T>
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
    complete: bool,
}

impl<T: GetId> Default for Primary<T> {
    fn default() -> Self {
        Self {
            inner: None,
            complete: true,
        }
    }
}

impl<T> Clone for Primary<T>
where
    T: GetId + Clone,
    PrimaryEvent<T>: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            complete: self.complete,
        }
    }
}

impl<T: GetId + Eq> Eq for Primary<T> {}

impl<T: GetId + PartialEq> PartialEq for Primary<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl<T> fmt::Debug for Primary<T>
where
    T: GetId + fmt::Debug,
    Vec<PrimaryEvent<T>>: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl<T: GetId> Primary<T>
where
    PrimaryEvent<T>: Sized,
{
    pub fn new(row: T) -> (Self, Changes<Self>)
    where
        T: Clone,
    {
        let mut result = Self {
            inner: None,
            complete: true,
        };

        let changes = result.create(row);
        (result, changes)
    }

    pub fn get(&self) -> &T {
        self.inner.as_ref().expect("not deleted")
    }

    pub fn try_get(&self) -> Option<&T> {
        self.inner.as_ref()
    }

    pub fn create(&mut self, row: T) -> Changes<Self>
    where
        T: Clone,
    {
        let id = row.get_id();
        Changes::once(self.mutate(Created(row), Deleted(id)))
    }

    pub fn set(&mut self, row: T) -> StdResult<Changes<Self>, NotFound<T>>
    where
        T: Clone,
    {
        if let Some(existing) = &self.inner {
            let existing = existing.clone();
            Ok(Changes::once(self.mutate(Updated(row), Updated(existing))))
        } else {
            Err(NotFound(row))
        }
    }

    pub fn update<F>(&mut self, f: F) -> StdResult<Changes<Self>, NotFound<()>>
    where
        F: FnOnce(&mut T),
        T: Clone,
    {
        if let Some(existing) = &self.inner {
            let mut modified = existing.clone();
            f(&mut modified);
            let existing = existing.clone();
            Ok(Changes::once(
                self.mutate(Updated(modified), Updated(existing)),
            ))
        } else {
            Err(NotFound(()))
        }
    }

    pub fn delete(&mut self) -> StdResult<Changes<Self>, NotFound<()>>
    where
        T: Clone,
    {
        if let Some(existing) = &self.inner {
            let id = existing.get_id();
            let existing = existing.clone();
            Ok(Changes::once(self.mutate(Deleted(id), Created(existing))))
        } else {
            Err(NotFound(()))
        }
    }
}

impl<T> Changable for Primary<T>
where
    T: GetId + Clone,
{
    type EventType = PrimaryEvent<T>;

    fn apply(&mut self, event: &Self::EventType) -> Self::EventType {
        match event {
            Created(x) => {
                self.inner = Some(x.clone());
                Deleted(x.get_id())
            }
            Updated(x) => {
                let old = self.inner.replace(x.clone());
                Updated(old.expect("Dev err: update before create"))
            }
            Deleted(_) => {
                let old = self.inner.take();
                Created(old.expect("Dev err: delete before create"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    fn setup() -> Primary<MyEntity> {
        let (result, _changes) = Primary::new(MyEntity {
            id: ID,
            name: "foo".into(),
        });

        result
    }

    #[test]
    fn should_set() {
        let mut sut = setup();

        let changes: Vec<_> = sut
            .set(MyEntity {
                id: ID,
                name: "bar".into(),
            })
            .unwrap()
            .into();

        assert_eq!(sut.get().name.as_str(), "bar");
        assert_eq!(
            changes,
            vec![BasicChange {
                redo: Updated(MyEntity {
                    id: ID,
                    name: "bar".into()
                }),
                undo: Updated(MyEntity {
                    id: ID,
                    name: "foo".into()
                })
            }]
        );
    }

    #[test]
    fn should_update() {
        let mut sut = setup();

        let changes: Vec<_> = sut.update(|x| x.name = "bar".into()).unwrap().into();

        assert_eq!(sut.get().name.as_str(), "bar");
        assert_eq!(
            changes,
            vec![BasicChange {
                redo: Updated(MyEntity {
                    id: ID,
                    name: "bar".into()
                }),
                undo: Updated(MyEntity {
                    id: ID,
                    name: "foo".into()
                })
            }]
        );
    }

    #[test]
    fn should_delete() {
        let mut sut = setup();

        let changes: Vec<_> = sut.delete().unwrap().into();

        assert_eq!(sut.try_get(), None);
        assert_eq!(
            changes,
            vec![BasicChange {
                redo: Deleted(
                    (MyEntity {
                        id: ID,
                        name: "foo".into()
                    })
                    .get_id()
                ),
                undo: Created(MyEntity {
                    id: ID,
                    name: "foo".into()
                })
            }]
        );
    }
}
