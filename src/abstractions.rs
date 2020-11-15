use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

pub fn changes<T: Changable>(event: T::EventType) -> Changes<T> {
    std::iter::once(event).collect()
}

pub struct Changes<T: Changable> {
    inner: SmallList<<T as Changable>::EventType>,
}

impl<T: Changable> Changes<T> {
    pub fn new() -> Self {
        Changes {
            inner: SmallList::new(),
        }
    }

    pub fn to(self, dest: &mut Self) {
        dest.extend(self)
    }

    pub fn push(&mut self, event: T::EventType) {
        self.inner.push(event)
    }

    pub fn ascend<F, O: Changable>(self, f: F) -> Changes<O>
    where
        F: Fn(T::EventType) -> O::EventType,
    {
        self.into_iter().map(f).collect::<Changes<O>>()
    }

    /*
     * TODO: remove becauseimmutable `f` causes issue
     */
    pub fn ascend_to<O: Changable, F>(self, f: F, dest: &mut Changes<O>)
    where
        F: Fn(T::EventType) -> O::EventType,
    {
        dest.extend(self.into_iter().map(f));
    }
}

trait ExtendTo: IntoIterator + Sized {
    fn extend_to<I: Extend<Self::Item>>(self, other: &mut I) {
        other.extend(self)
    }
}

impl<T: Changable> Default for Changes<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Changable> Clone for Changes<T>
where
    T::EventType: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: Changable> PartialEq for Changes<T>
where
    T::EventType: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&self.inner, &other.inner)
    }
}

impl<T: Changable> Eq for Changes<T> where T::EventType: Eq {}

impl<T: Changable> Debug for Changes<T>
where
    T::EventType: Debug,
{
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("Changes")
            .field("items", &self.inner)
            .finish()
    }
}

impl<T: Changable> Extend<T::EventType> for Changes<T> {
    fn extend<I: IntoIterator<Item = T::EventType>>(&mut self, iter: I) {
        self.inner.extend(iter)
    }
}

impl<T: Changable> Into<Vec<T::EventType>> for Changes<T> {
    fn into(self) -> Vec<T::EventType> {
        self.inner.into_iter().collect()
    }
}

impl<T: Changable> std::iter::FromIterator<T::EventType> for Changes<T> {
    fn from_iter<I: IntoIterator<Item = T::EventType>>(iter: I) -> Self {
        Self {
            inner: iter.into_iter().collect(),
        }
    }
}

impl<T: Changable> std::iter::IntoIterator for Changes<T> {
    type Item = T::EventType;
    type IntoIter = <Vec<T::EventType> as std::iter::IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

struct SmallList<T> {
    inner: Vec<T>,
}

impl<T> Debug for SmallList<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_list().entries(&self.inner).finish()
    }
}

impl<T> SmallList<T> {
    pub fn new() -> Self {
        Self { inner: Vec::new() }
    }

    pub fn push(&mut self, item: T) {
        self.inner.push(item)
    }
}

impl<T: Clone> Clone for SmallList<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Extend<T> for SmallList<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.inner.extend(iter)
    }
}

impl<T> Into<Vec<T>> for SmallList<T> {
    fn into(self) -> Vec<T> {
        self.into_iter().collect()
    }
}

impl<T> std::iter::FromIterator<T> for SmallList<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self {
            inner: iter.into_iter().collect(),
        }
    }
}

impl<T: PartialEq> PartialEq for SmallList<T> {
    fn eq(&self, other: &Self) -> bool {
        return PartialEq::eq(&self.inner, &other.inner);
    }
}

impl<T: Eq> Eq for SmallList<T> {}

impl<T> std::iter::IntoIterator for SmallList<T> {
    type Item = T;
    type IntoIter = <Vec<T> as std::iter::IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

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
    marker: std::marker::PhantomData<T>,
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
        Self {
            id,
            marker: Default::default(),
        }
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

pub enum EventMergeResult {
    Combined,
    Annihilated,
}

pub trait Changable {
    type EventType;

    fn apply(&mut self, event: Self::EventType);
}

pub trait Streamable: Changable + Sized {
    fn new_incomplete() -> Self;

    fn mark_complete(&mut self);

    fn stream_to<S>(&mut self, stream: &mut S)
    where
        S: StreamEvents<Self::EventType>;

    fn commit_changes(&mut self) -> Vec<Self::EventType> {
        let mut result = Vec::new();
        self.stream_to(&mut result);
        result
    }

    fn load<I>(events: I) -> crate::result::Result<Self>
    where
        I: IntoIterator<Item = Self::EventType>,
        Self::EventType: Clone,
    {
        let mut result = Self::new_incomplete();
        for e in events {
            result.apply(e);
        }

        result.mark_complete();

        Ok(result)
    }
}

pub trait StreamEvents<TEvent>: Sized {
    fn stream<I>(&mut self, events: I)
    where
        I: IntoIterator<Item = TEvent>;

    fn flush<S, U, F>(&mut self, s: &mut S, f: F)
    where
        S: Streamable<EventType = U>,
        F: Fn(U) -> TEvent,
    {
        s.stream_to(&mut StreamAdapter::new(self, f))
    }
}

impl<S, TEvent> StreamEvents<TEvent> for &mut S
where
    S: StreamEvents<TEvent>,
{
    fn stream<I>(&mut self, events: I)
    where
        I: IntoIterator<Item = TEvent>,
    {
        (*self).stream(events);
    }
}

impl<TEvent> StreamEvents<TEvent> for Vec<TEvent> {
    fn stream<I>(&mut self, events: I)
    where
        I: IntoIterator<Item = TEvent>,
    {
        self.extend(events);
    }
}

pub struct StreamAdapter<TInner, F>(TInner, F);

impl<TInner, F> StreamAdapter<TInner, F> {
    pub fn new(original: TInner, f: F) -> Self {
        Self(original, f)
    }
}

impl<TInnerEvent, TEvent, TInner, F> StreamEvents<TEvent> for StreamAdapter<TInner, F>
where
    TInner: StreamEvents<TInnerEvent>,
    F: Fn(TEvent) -> TInnerEvent,
{
    fn stream<I>(&mut self, events: I)
    where
        I: IntoIterator<Item = TEvent>,
    {
        self.0.stream(events.into_iter().map(&self.1))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn use_cases() {}
}
