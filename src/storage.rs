use crate::changable::Changable;
use crate::identifiable::GetId;
use crate::result::Result;
use crate::streamable::{Streamable, Unstreamable};
use crate::streaming::StreamAdapter;
use std::error::Error as StdError;
use std::fmt;
use std::result::Result as StdResult;

pub trait Load<T>
where
    T: GetId,
{
    fn load(&mut self, id: &T::Id) -> Result<T>;
}

pub trait Save<T> {
    fn save(&mut self, root: T) -> StdResult<(), Box<dyn StdError>>;
}

struct EventEnvelope<T: GetId, TEvent> {
    pub id: T::Id,
    pub event: TEvent,
}

impl<T: GetId, TEvent> EventEnvelope<T, TEvent> {
    fn new(id: T::Id, event: TEvent) -> Self {
        Self { id, event }
    }
}

impl<T, TEvent> Clone for EventEnvelope<T, TEvent>
where
    T: GetId,
    T::Id: Clone,
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
    T: GetId,
    T::Id: fmt::Debug,
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
    T: GetId,
{
    events: Vec<EventEnvelope<T, TEvent>>,
}

impl<T, TEvent> InMemoryStorage<T, TEvent>
where
    T: Changable<EventType = TEvent> + GetId,
    T::Id: Clone,
{
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    fn select_events<'a>(&'a self, id: &'a T::Id) -> impl 'a + Iterator<Item = TEvent>
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
    T: GetId,
    TEvent: fmt::Debug,
    T::Id: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("InMemoryStorage")
            .field("events", &self.events)
            .finish()
    }
}

impl<T, TEvent> Load<T> for InMemoryStorage<T, TEvent>
where
    T: Unstreamable<EventType = TEvent> + GetId,
    TEvent: Clone,
    T::Id: Clone,
{
    fn load(&mut self, id: &T::Id) -> Result<T> {
        let events = self.select_events(id);
        T::load(events)
    }
}

impl<T, TEvent> Save<T> for InMemoryStorage<T, TEvent>
where
    T: Streamable<EventType = TEvent> + GetId,
    T::Id: Clone,
{
    fn save(&mut self, mut root: T) -> StdResult<(), Box<dyn StdError>> {
        let id: T::Id = root.get_id();
        let to_envelope = |e| EventEnvelope::new(id.clone(), e);
        let mut adapter = StreamAdapter::new(&mut self.events, to_envelope);
        root.stream_to(&mut adapter)
    }
}
