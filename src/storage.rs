use crate::changable::Changable;
use crate::identifiable::{Id, Identifiable};
use crate::result::Result;
use crate::streamable::{Streamable, SupportsDeletion, Unstreamable};
use crate::streaming::StreamAdapter;
use std::error::Error as StdError;
use std::fmt;
use std::hash::Hash;
use std::result::Result as StdResult;

pub trait Load<T>
where
    T: Identifiable,
{
    fn load(&mut self, id: &Id<T>) -> Result<T>;

    fn load_all(&mut self) -> Result<Vec<T>>;
}

pub trait Save<T> {
    fn save(&mut self, root: T) -> StdResult<(), Box<dyn StdError>>;
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
    T: Identifiable,
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

impl<T, TEvent> fmt::Debug for EventEnvelope<T, TEvent>
where
    T: Identifiable,
    Id<T>: fmt::Debug,
    TEvent: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("EventEnvelope")
            .field("id", &self.id)
            .field("events", &self.event)
            .finish()
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
    T: Changable<EventType = TEvent> + Identifiable,
    Id<T>: Clone,
{
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    fn select_events<'a>(&'a self, id: &'a Id<T>) -> impl 'a + Iterator<Item = TEvent>
    where
        TEvent: Clone,
    {
        /*
        let selected: Vec<_> = self
            .events
            .iter()
            .filter(move |x| &x.id == id)
            .map(|x| &x.event)
            .cloned()
            .collect();

        println!("** selected events = {:#?}", selected);
        println!("** all events = {:#?}", self.events);
        */
        self.events
            .iter()
            .filter(move |x| &x.id == id)
            .map(|x| &x.event)
            .cloned()
    }
}

impl<T, TEvent> fmt::Debug for InMemoryStorage<T, TEvent>
where
    T: Identifiable,
    TEvent: fmt::Debug,
    Id<T>: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("InMemoryStorage")
            .field("events", &self.events)
            .finish()
    }
}

impl<T, TEvent> Load<T> for InMemoryStorage<T, TEvent>
where
    T: Unstreamable<EventType = TEvent> + Identifiable,
    TEvent: Clone + SupportsDeletion,
    Id<T>: Hash + Clone,
{
    fn load(&mut self, id: &Id<T>) -> Result<T> {
        let events = self.select_events(id);
        T::load(events)
    }

    fn load_all(&mut self) -> Result<Vec<T>> {
        let all_events = self.events.iter().map(|x| (x.id.clone(), x.event.clone()));

        T::load_many(all_events)
    }
}

impl<T, TEvent> Save<T> for InMemoryStorage<T, TEvent>
where
    T: Streamable<EventType = TEvent> + Identifiable,
    Id<T>: Clone,
{
    fn save(&mut self, mut root: T) -> StdResult<(), Box<dyn StdError>> {
        let id = root.id();
        let to_envelope = |e| EventEnvelope::new(id.clone(), e);
        let mut adapter = StreamAdapter::new(&mut self.events, to_envelope);
        root.stream_to(&mut adapter)
    }
}
