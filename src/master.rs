use crate::changable::Changable;
use crate::change_abs::AppliedChange;
use crate::historic::Historic;
use crate::identifiable::*;
use crate::result::NotFound;
use crate::FullChanges;
use std::cmp::{Eq, PartialEq};
use std::fmt;
use std::marker;
use std::result::Result as StdResult;
use MasterEvent::*;

pub enum MasterEvent<T>
where
    T: GetId,
{
    Created(T),
    Updated(T),
    Deleted(Id<T::IdentifiableType>),
}

impl<T> fmt::Debug for MasterEvent<T>
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

impl<T> Clone for MasterEvent<T>
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

impl<T> Eq for MasterEvent<T> where T: GetId + Eq {}

impl<T> PartialEq for MasterEvent<T>
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

pub struct Master<T: GetId, C = FullChanges<MasterEvent<T>>> {
    inner: Option<T>,
    marker: marker::PhantomData<C>,
}

impl<T: GetId, C> Default for Master<T, C> {
    fn default() -> Self {
        Self {
            inner: None,
            marker: marker::PhantomData,
        }
    }
}

impl<T, C> Clone for Master<T, C>
where
    T: GetId + Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            marker: marker::PhantomData,
        }
    }
}

impl<T: GetId + Eq, C> Eq for Master<T, C> {}

impl<T: GetId + PartialEq, C> PartialEq for Master<T, C> {
    fn eq(&self, other: &Self) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl<T, C> fmt::Debug for Master<T, C>
where
    T: GetId + fmt::Debug,
    Vec<MasterEvent<T>>: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl<T: GetId, C> Master<T, C>
where
    C: AppliedChange<MasterEvent<T>>,
    MasterEvent<T>: Sized,
    Id<<T as GetId>::IdentifiableType>: Clone,
{
    pub fn new(row: T) -> (Self, C) {
        let mut result = Self {
            inner: None,
            marker: marker::PhantomData,
        };

        let changes = result.create(row);
        (result, changes)
    }

    pub fn try_get_id(&self) -> Option<Id<T::IdentifiableType>> {
        self.inner.as_ref().map(GetId::get_id)
    }

    pub fn get(&self) -> &T {
        self.inner.as_ref().expect("not deleted")
    }

    pub fn try_get(&self) -> Option<&T> {
        self.inner.as_ref()
    }

    pub fn create(&mut self, row: T) -> C {
        self.applied(Created(row))
    }

    pub fn set(&mut self, row: T) -> StdResult<C, NotFound<T>> {
        if let Some(_) = &self.inner {
            Ok(self.applied(Updated(row)))
        } else {
            Err(NotFound(row))
        }
    }

    pub fn update<F>(&mut self, f: F) -> StdResult<C, NotFound<()>>
    where
        F: FnOnce(&mut T),
        T: Clone,
    {
        if let Some(existing) = &self.inner {
            let mut modified = existing.clone();
            f(&mut modified);
            Ok(self.applied(Updated(modified)))
        } else {
            Err(NotFound(()))
        }
    }

    pub fn delete(&mut self) -> StdResult<C, NotFound<()>> {
        if let Some(existing) = &self.inner {
            let id = existing.get_id();
            Ok(self.applied(Deleted(id)))
        } else {
            Err(NotFound(()))
        }
    }
}

impl<T, C> Historic for Master<T, C>
where
    T: GetId,
    Id<T::IdentifiableType>: Clone,
{
    type EventType = MasterEvent<T>;
}

impl<T, C> Changable for Master<T, C>
where
    T: GetId,
    Id<T::IdentifiableType>: Clone,
{
    fn apply(&mut self, event: Self::EventType) -> Self::EventType {
        match event {
            Created(x) => {
                let id = x.get_id();
                self.inner = Some(x);
                Deleted(id)
            }
            Updated(x) => {
                let old = self.inner.replace(x);
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
    use crate::changes::FullChange;
    use crate::changes::FullChanges;
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

    fn setup() -> Master<MyEntity> {
        let (result, _changes): (_, FullChanges<_>) = Master::new(MyEntity {
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
            vec![FullChange::new(
                Updated(MyEntity {
                    id: ID,
                    name: "bar".into()
                }),
                Updated(MyEntity {
                    id: ID,
                    name: "foo".into()
                })
            )]
        );
    }

    #[test]
    fn should_update() {
        let mut sut = setup();

        let changes: Vec<_> = sut.update(|x| x.name = "bar".into()).unwrap().into();

        assert_eq!(sut.get().name.as_str(), "bar");
        assert_eq!(
            changes,
            vec![FullChange::new(
                Updated(MyEntity {
                    id: ID,
                    name: "bar".into()
                }),
                Updated(MyEntity {
                    id: ID,
                    name: "foo".into()
                })
            )]
        );
    }

    #[test]
    fn should_delete() {
        let mut sut = setup();

        let changes: Vec<_> = sut.delete().unwrap().into();

        assert_eq!(sut.try_get(), None);
        assert_eq!(
            changes,
            vec![FullChange::new(
                Deleted(
                    (MyEntity {
                        id: ID,
                        name: "foo".into()
                    })
                    .get_id()
                ),
                Created(MyEntity {
                    id: ID,
                    name: "foo".into()
                })
            )]
        );
    }
}
