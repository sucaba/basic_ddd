use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use std::fmt::{Debug, Formatter};

//pub type Result<T> = std::result::Result<T, Error>;

pub trait HasId: Sized {
    type IdType;

    fn id(&self) -> Id<Self>;
}

pub trait HasOwner {
    type OwnerType: HasId;
}

pub struct Id<T: HasId> {
    id: T::IdType,
    marker: std::marker::PhantomData<T>,
}

impl<T: HasId> Id<T> {
    pub fn new(id: T::IdType) -> Self {
        Self {
            id,
            marker: Default::default(),
        }
    }

    pub fn id(&self) -> &T::IdType {
        &self.id
    }
}

impl<T: HasId> Clone for Id<T>
where
    T::IdType: Clone,
{
    fn clone(&self) -> Self {
        Id::new(self.id.clone())
    }
}

impl<T: HasId> Debug for Id<T>
where
    T::IdType: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.id.fmt(f)
    }
}

impl<T: HasId> Ord for Id<T>
where
    T::IdType: Ord,
{
    fn cmp(&self, y: &Id<T>) -> Ordering {
        Ord::cmp(&self.id, &y.id)
    }
}

impl<T: HasId> PartialOrd for Id<T>
where
    T::IdType: PartialOrd,
{
    fn partial_cmp(&self, y: &Id<T>) -> Option<Ordering> {
        self.id.partial_cmp(&y.id)
    }
}

impl<T: HasId> Eq for Id<T> where T::IdType: Eq {}

impl<T: HasId> PartialEq for Id<T>
where
    T::IdType: PartialEq,
{
    fn eq(&self, y: &Id<T>) -> bool {
        self.id.eq(&y.id)
    }
}

pub trait Streamable {
    type EventType;

    fn stream_to<S>(&mut self, stream: &mut S)
    where
        S: StreamEvents<Self::EventType>;

    fn commit_changes(&mut self) -> Vec<Self::EventType> {
        let mut result = Vec::new();
        self.stream_to(&mut result);
        result
    }
}

pub trait StreamEvents<TEvent>: Sized {
    fn stream<I>(&mut self, events: I)
    where
        I: IntoIterator<Item = TEvent>;

    fn stream_one(&mut self, event: TEvent) {
        self.stream(std::iter::once(event));
    }

    fn flush<S, U>(&mut self, s: &mut S)
    where
        S: Streamable<EventType = U>,
        TEvent: From<U>,
    {
        s.stream_to(&mut StreamAdapter::new(self))
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

pub trait AdaptStream<SE>
where
    Self: Sized + StreamEvents<SE>,
{
    fn adapt<'a>(&'a mut self) -> StreamAdapter<'a, SE, Self>;
}

impl<SE, S> AdaptStream<SE> for S
where
    S: StreamEvents<SE>,
{
    fn adapt<'a>(&'a mut self) -> StreamAdapter<'a, SE, Self> {
        StreamAdapter::new(self)
    }
}

pub struct StreamAdapter<'a, SE, S>(&'a mut S, std::marker::PhantomData<SE>)
where
    S: StreamEvents<SE>;

impl<'a, SE, S> StreamAdapter<'a, SE, S>
where
    S: StreamEvents<SE>,
{
    pub fn new(original: &'a mut S) -> Self {
        Self(original, std::marker::PhantomData)
    }
}

impl<'a, SE, S, E> StreamEvents<E> for StreamAdapter<'a, SE, S>
where
    S: StreamEvents<SE>,
    SE: From<E>,
{
    fn stream<I>(&mut self, events: I)
    where
        I: IntoIterator<Item = E>,
    {
        self.0.stream(events.into_iter().map(|e| e.into()))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn use_cases() {}
}
