use crate::abstractions::{Id, Identifiable, StreamAdapter, Streamable};
use crate::result::Result;

pub trait Load<T>
where
    T: Identifiable,
{
    fn load(&mut self, id: &Id<T>) -> Result<T>;
}

pub trait Save<T> {
    fn save(&mut self, root: T) -> Result<()>;
}

struct EventEnvelope<T: Identifiable, TEvent> {
    pub id: Id<T>,
    pub event: TEvent,
}

impl<T: Identifiable, TEvent> EventEnvelope<T, TEvent> {
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
    T: Identifiable,
{
    events: Vec<EventEnvelope<T, TEvent>>,
}

impl<T, TEvent> InMemoryStorage<T, TEvent>
where
    T: Streamable<EventType = TEvent> + Identifiable,
    TEvent: 'static + std::fmt::Debug + Clone,
    Id<T>: Clone,
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
}

impl<T, TEvent> Load<T> for InMemoryStorage<T, TEvent>
where
    T: Streamable<EventType = TEvent> + Identifiable,
    TEvent: 'static + std::fmt::Debug + Clone,
    Id<T>: Clone,
{
    fn load(&mut self, id: &Id<T>) -> Result<T> {
        let events = self.select_events(id);
        T::load(events)
    }
}

impl<T, TEvent> Save<T> for InMemoryStorage<T, TEvent>
where
    T: Streamable<EventType = TEvent> + Identifiable,
    Id<T>: Clone,
{
    fn save(&mut self, mut root: T) -> Result<()> {
        let id = root.id();
        let to_envelope = |e| EventEnvelope::new(id.clone(), e);
        let mut adapter = StreamAdapter::new(&mut self.events, to_envelope);
        root.stream_to(&mut adapter);
        Ok(())
    }
}
