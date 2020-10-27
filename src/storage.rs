use crate::abstractions::{Id, Identifiable, Streamable};
use crate::result::Result;
use std::hash::Hash;
use std::marker::PhantomData;

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
    marker: PhantomData<T>,
}

impl<T, TEvent> InMemoryStorage<T, TEvent>
where
    T: Streamable<EventType = TEvent> + Identifiable,
    TEvent: std::fmt::Debug + Clone,
    Id<T>: Hash + Clone,
{
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            marker: PhantomData,
        }
    }

    fn select_events<'a>(&'a self, id: &'a Id<T>) -> impl 'a + Iterator<Item = TEvent> {
        self.events
            .iter()
            .filter(move |x| &x.id == id)
            .map(|x| x.event.clone())
    }

    pub fn load(&mut self, id: &Id<T>) -> Result<T> {
        let events = self.select_events(id);
        T::load(events)
    }

    pub fn save(&mut self, mut root: T) -> Result<()> {
        let id = root.id();
        let mut events = Vec::new();
        root.stream_to(&mut events);
        self.events.extend(
            events
                .into_iter()
                .map(|e| EventEnvelope::new(id.clone(), e)),
        );
        Ok(())
    }
}
