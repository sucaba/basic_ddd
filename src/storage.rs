use crate::changable::Changable;
use crate::identifiable::{GetId, Id};
use crate::result::Result;
use crate::streamable::{Streamable, StreamableInContext, SupportsDeletion, Unstreamable};
use crate::streaming::StreamAdapter;
use std::error::Error as StdError;
use std::fmt;
use std::hash::Hash;
use std::result::Result as StdResult;

struct EventEnvelope<T: GetId, TEvent> {
    pub id: Id<T::IdentifiableType>,
    pub event: TEvent,
}

impl<T: GetId, TEvent> EventEnvelope<T, TEvent> {
    fn new(id: Id<T::IdentifiableType>, event: TEvent) -> Self {
        Self { id, event }
    }
}

impl<T, TEvent> Clone for EventEnvelope<T, TEvent>
where
    T: GetId,
    Id<T::IdentifiableType>: Clone,
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
    Id<T::IdentifiableType>: fmt::Debug,
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
    Id<T::IdentifiableType>: Clone,
{
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    fn select_events<'a>(
        &'a self,
        id: &'a Id<T::IdentifiableType>,
    ) -> impl 'a + Iterator<Item = TEvent>
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

    pub fn load(&mut self, id: &Id<T::IdentifiableType>) -> Result<T>
    where
        T: Unstreamable<EventType = TEvent>,
        TEvent: Clone,
    {
        let events = self.select_events(id);
        T::load(events)
    }

    pub fn load_all(&mut self) -> Result<Vec<T>>
    where
        T: Unstreamable<EventType = TEvent>,
        TEvent: Clone + SupportsDeletion,
        Id<T::IdentifiableType>: Hash,
    {
        let all_events = self.events.iter().map(|x| (x.id.clone(), x.event.clone()));

        T::load_many(all_events)
    }

    pub fn save(&mut self, mut root: T) -> StdResult<(), Box<dyn StdError>>
    where
        T: Streamable<EventType = TEvent>,
    {
        let id = root.get_id();
        let to_envelope = |e| EventEnvelope::new(id.clone(), e);
        let mut adapter = StreamAdapter::new(&mut self.events, to_envelope);
        root.stream_to(&mut adapter)
    }

    pub fn save_in_context<TCtx>(
        &mut self,
        ctx: &mut TCtx,
        root: &mut T,
    ) -> StdResult<(), Box<dyn StdError>>
    where
        T: StreamableInContext<TCtx>,
    {
        let id = root.get_id();
        let to_envelope = |e| EventEnvelope::new(id.clone(), e);
        let mut adapter = StreamAdapter::new(&mut self.events, to_envelope);
        root.stream_in_context_to(ctx, &mut adapter)
    }
}

impl<T, TEvent> fmt::Debug for InMemoryStorage<T, TEvent>
where
    T: GetId,
    TEvent: fmt::Debug,
    Id<T::IdentifiableType>: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("InMemoryStorage")
            .field("events", &self.events)
            .finish()
    }
}
