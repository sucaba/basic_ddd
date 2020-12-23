use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};

use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

pub trait Identifiable: Sized {
    type IdType: Eq;

    fn id(&self) -> Id<Self>;
}

pub trait GetId {
    type IdentifiableType: Identifiable;

    fn get_id(&self) -> Id<Self::IdentifiableType>;
}

impl<T> GetId for T
where
    T: Identifiable,
{
    type IdentifiableType = T;

    fn get_id(&self) -> Id<Self::IdentifiableType> {
        Identifiable::id(self)
    }
}

impl<T> GetId for Rc<T>
where
    T: GetId,
{
    type IdentifiableType = T::IdentifiableType;

    fn get_id(&self) -> Id<Self::IdentifiableType> {
        GetId::get_id(std::ops::Deref::deref(self))
    }
}

impl<T> GetId for Id<T>
where
    T: Identifiable,
    Self: Clone,
{
    type IdentifiableType = T;

    fn get_id(&self) -> Id<Self::IdentifiableType> {
        self.clone()
    }
}

pub trait Owned {
    type OwnerType: Identifiable;
}

pub struct Id<T: Identifiable> {
    id: T::IdType,
}

impl<T: Identifiable> Hash for Id<T>
where
    T::IdType: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T: Identifiable> Id<T> {
    pub fn new(id: T::IdType) -> Self {
        Self { id }
    }

    pub fn id(&self) -> &T::IdType {
        &self.id
    }

    pub fn convert<U>(self) -> Id<U>
    where
        U: Identifiable<IdType = T::IdType>,
    {
        Id::new(self.id)
    }
}

impl<T: Identifiable> Copy for Id<T>
where
    Self: Clone,
    T::IdType: Copy,
{
}

impl<T: Identifiable> Clone for Id<T>
where
    T::IdType: Clone,
{
    fn clone(&self) -> Self {
        Id::new(self.id.clone())
    }
}

impl<T: Identifiable> Debug for Id<T>
where
    T::IdType: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.id.fmt(f)
    }
}

impl<T: Identifiable> Ord for Id<T>
where
    T::IdType: Ord,
{
    fn cmp(&self, y: &Id<T>) -> Ordering {
        Ord::cmp(&self.id, &y.id)
    }
}

impl<T: Identifiable> PartialOrd for Id<T>
where
    T::IdType: PartialOrd,
{
    fn partial_cmp(&self, y: &Id<T>) -> Option<Ordering> {
        self.id.partial_cmp(&y.id)
    }
}

impl<T: Identifiable> Eq for Id<T> {}

impl<T: Identifiable> PartialEq for Id<T> {
    fn eq(&self, y: &Id<T>) -> bool {
        self.id.eq(&y.id)
    }
}
