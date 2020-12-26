use super::changable::Changable;
use super::change_abs::AppliedChange;
use super::identifiable::*;
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
}

impl<T: GetId> Default for Primary<T> {
    fn default() -> Self {
        Self { inner: None }
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
    Id<<T as GetId>::IdentifiableType>: Clone,
{
    pub fn new<C>(row: T) -> (Self, C)
    where
        C: AppliedChange<PrimaryEvent<T>>,
        T: Clone,
    {
        let mut result = Self { inner: None };

        let changes = result.create(row);
        (result, changes)
    }

    pub fn get(&self) -> &T {
        self.inner.as_ref().expect("not deleted")
    }

    pub fn try_get(&self) -> Option<&T> {
        self.inner.as_ref()
    }

    pub fn create<C>(&mut self, row: T) -> C
    where
        C: AppliedChange<PrimaryEvent<T>>,
        T: Clone,
    {
        self.applied(Created(row))
    }

    pub fn set<C>(&mut self, row: T) -> StdResult<C, NotFound<T>>
    where
        C: AppliedChange<PrimaryEvent<T>>,
        T: Clone,
    {
        if let Some(_) = &self.inner {
            Ok(self.applied(Updated(row)))
        } else {
            Err(NotFound(row))
        }
    }

    pub fn update<F, C>(&mut self, f: F) -> StdResult<C, NotFound<()>>
    where
        C: AppliedChange<PrimaryEvent<T>>,
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

    pub fn delete<C>(&mut self) -> StdResult<C, NotFound<()>>
    where
        C: AppliedChange<PrimaryEvent<T>>,
        T: Clone,
    {
        if let Some(existing) = &self.inner {
            let id = existing.get_id();
            Ok(self.applied(Deleted(id)))
        } else {
            Err(NotFound(()))
        }
    }
}

impl<T> Changable for Primary<T>
where
    T: GetId + Clone,
    Id<T::IdentifiableType>: Clone,
{
    type EventType = PrimaryEvent<T>;

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

    fn setup() -> Primary<MyEntity> {
        let (result, _changes): (_, FullChanges<_>) = Primary::new(MyEntity {
            id: ID,
            name: "foo".into(),
        });

        result
    }

    #[test]
    fn should_set() {
        let mut sut = setup();

        let changes: Vec<_> = sut
            .set::<FullChanges<_>>(MyEntity {
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

        let changes: Vec<_> = sut
            .update::<_, FullChanges<_>>(|x| x.name = "bar".into())
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
    fn should_delete() {
        let mut sut = setup();

        let changes: Vec<_> = sut.delete::<FullChanges<_>>().unwrap().into();

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
