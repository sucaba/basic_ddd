use crate::abstractions::{Id, Identifiable, StreamAdapter, Streamable};
use crate::result::Result;
use std::hash::Hash;

pub trait Load<T>
where
    T: Streamable + Identifiable,
{
    fn load(&mut self, id: &Id<T>) -> Result<T>;
}

pub trait Save<T>
where
    T: Streamable + Identifiable,
{
    fn save(&mut self, root: T) -> Result<()>;
}

pub trait StoreEvents<T>: Load<T> + Save<T>
where
    T: Streamable + Identifiable,
{
}

struct EventEnvelope<T, TEvent>
where
    T: Streamable<EventType = TEvent> + Identifiable,
    Id<T>: Clone,
    TEvent: Clone,
{
    pub id: Id<T>,
    pub event: TEvent,
}

impl<T, TEvent> EventEnvelope<T, TEvent>
where
    T: Streamable<EventType = TEvent> + Identifiable,
    Id<T>: Clone,
    TEvent: Clone,
{
    fn new(id: Id<T>, event: TEvent) -> Self {
        Self { id, event }
    }
}

impl<T, TEvent> Clone for EventEnvelope<T, TEvent>
where
    T: Streamable<EventType = TEvent> + Identifiable,
    Id<T>: Clone,
    TEvent: Clone,
{
    fn clone(&self) -> Self {
        EventEnvelope {
            id: Clone::clone(&self.id),
            event: Clone::clone(&self.event),
        }
    }
}

pub struct InMemoryStorage<T, TEvent>
where
    T: Streamable<EventType = TEvent> + Identifiable,
    Id<T>: Clone,
    TEvent: Clone,
{
    events: Vec<EventEnvelope<T, TEvent>>,
}

impl<T, TEvent> InMemoryStorage<T, TEvent>
where
    T: Streamable<EventType = TEvent> + Identifiable,
    TEvent: 'static + std::fmt::Debug + Clone,
    Id<T>: Hash + Clone,
{
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    fn select_events<'a>(&'a self, id: &'a Id<T>) -> impl 'a + Iterator<Item = &TEvent> {
        self.events
            .iter()
            .filter(move |x| &x.id == id)
            .map(|x| &x.event)
    }

    pub fn load(&mut self, id: &Id<T>) -> Result<T> {
        let events = self.select_events(id);
        T::load(events)
    }

    pub fn save(&mut self, mut root: T) -> Result<()> {
        let id = root.id();
        let to_envelope = |e| EventEnvelope::new(id.clone(), e);
        let mut adapter =
            StreamAdapter::new(&mut self.events, to_envelope);
        root.stream_to(&mut adapter);
        Ok(())
    }
}
