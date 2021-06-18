use super::changable::Changable;
use crate::contextual::Contextual;
use crate::historic::Historic;
use crate::streaming::*;
use std::hash::Hash;
use std::{collections::HashMap, error::Error};

pub trait Streamable: Historic {
    fn stream_to<S>(&mut self, stream: &mut S) -> Result<usize, Box<dyn Error>>
    where
        S: Stream<Self::EventType>;

    // TODO: Result<Vec<..>>
    fn take_changes(&mut self) -> Vec<Self::EventType> {
        let mut result = Vec::new();
        self.stream_to(&mut result).unwrap();
        result
    }
}

#[non_exhaustive]
pub enum EventKind {
    Creation,
    Deletion,
    Other,
}

pub trait KindOfEvent {
    fn kind_of_event(&self) -> EventKind;
}

pub trait StreamableInContext<TCtx>: Historic {
    fn stream_in_context_to<S>(
        &mut self,
        context: &mut TCtx,
        stream: &mut S,
    ) -> Result<usize, Box<dyn Error>>
    where
        S: Stream<Self::EventType>;

    fn take_changes_in_context(&mut self, context: &mut TCtx) -> Vec<Self::EventType> {
        let mut result = Vec::new();
        let _count = self.stream_in_context_to(context, &mut result).unwrap();
        result
    }
}

impl<T: Historic, TCtx> Historic for Contextual<T, TCtx> {
    type EventType = T::EventType;
}

impl<T: Changable, TCtx> Changable for Contextual<T, TCtx> {
    fn apply(&mut self, event: Self::EventType) -> Self::EventType {
        self.subject.apply(event)
    }
}

impl<T, TCtx> Streamable for Contextual<T, TCtx>
where
    T: StreamableInContext<TCtx>,
{
    fn stream_to<S>(&mut self, stream: &mut S) -> Result<usize, Box<dyn Error>>
    where
        S: Stream<Self::EventType>,
    {
        self.subject.stream_in_context_to(&mut self.context, stream)
    }
}

pub trait Unstreamable: Changable + Default + Sized {
    fn load<'a, I>(events: I) -> crate::result::Result<Self>
    where
        I: IntoIterator<Item = Self::EventType>;

    fn load_many<I, ID>(events: I) -> crate::result::Result<Vec<Self>>
    where
        Self::EventType: KindOfEvent,
        I: IntoIterator<Item = (ID, Self::EventType)>,
        ID: Hash + Eq;
}

impl<T> Unstreamable for T
where
    T: Sized + Default + Changable,
{
    fn load<I>(events: I) -> crate::result::Result<Self>
    where
        I: IntoIterator<Item = Self::EventType>,
    {
        let mut result = Self::default();
        for e in events {
            let _non_undoable_change = result.apply(e);
        }

        Ok(result)
    }

    fn load_many<I, ID>(events: I) -> crate::result::Result<Vec<Self>>
    where
        Self::EventType: KindOfEvent,
        I: IntoIterator<Item = (ID, Self::EventType)>,
        ID: Hash + Eq,
    {
        let mut map = HashMap::<ID, Self>::new();
        for (id, e) in events {
            match e.kind_of_event() {
                EventKind::Creation => {
                    let aggregate = map
                        .entry(id)
                        .and_modify(|e| {
                            std::mem::take(e);
                        })
                        .or_default();
                    let _non_undoable_change = aggregate.apply(e);
                }
                EventKind::Deletion => {
                    let _ = map.remove_entry(&id);
                }
                EventKind::Other => {
                    if let std::collections::hash_map::Entry::Occupied(mut occupied) = map.entry(id)
                    {
                        let aggregate = occupied.get_mut();
                        let _non_undoable_change = aggregate.apply(e);
                    }
                }
            }
        }

        Ok(map.into_iter().map(|(_key, aggregate)| aggregate).collect())
    }
}

#[cfg(test)]
mod tests {
    use std::mem;

    use super::*;
    use crate::contextual::InContext;
    use crate::identifiable::{Id, Identifiable};
    use pretty_assertions::assert_eq;
    use MyEvent::*;
    use MyUnstreamableEvent::*;

    // Not really test for this module but rather a use case for implementing `Streamable` for
    // `Contextual`
    #[test]
    fn stream_in_context() {
        let entity_to_stream = MyStreamable;
        let context = MyContext { name: "exotic" };

        let mut sut = entity_to_stream.in_context(context);
        let mut stream = Vec::new();
        let count = sut.stream_to(&mut stream).unwrap();

        assert_eq!(stream, vec![Captured(MyContext { name: "exotic" })]);
        assert_eq!(1, count);
    }

    #[test]
    fn should_load_multiple() {
        let events = vec![
            (42, Created(Id::new(42), "red")),
            (13, Created(Id::new(13), "green")),
        ];

        let mut loaded = MyUnstreamable::load_many(events);
        if let Ok(entries) = &mut loaded {
            entries.sort_by_key(|x| x.1);
        }

        assert_eq!(
            loaded,
            Ok(vec![MyUnstreamable(13, "green"), MyUnstreamable(42, "red")])
        );
    }

    #[test]
    fn should_omit_deleted_when_loading_multiple() {
        let events = vec![
            (42, Created(Id::new(42), "red")),
            (13, Created(Id::new(13), "green")),
            (42, Deleted(Id::new(42))),
        ];

        let loaded = MyUnstreamable::load_many(events);

        assert_eq!(loaded, Ok(vec![MyUnstreamable(13, "green")]));
    }

    #[derive(Default, Debug, Eq, PartialEq)]
    struct MyUnstreamable(i32, &'static str);

    impl Identifiable for MyUnstreamable {
        type IdType = i32;

        fn id(&self) -> Id<Self> {
            Id::new(self.0)
        }
    }

    impl KindOfEvent for MyUnstreamableEvent {
        fn kind_of_event(&self) -> EventKind {
            match self {
                MyUnstreamableEvent::Created(_, _) => EventKind::Creation,
                MyUnstreamableEvent::Deleted(_) => EventKind::Deletion,
            }
        }
    }

    impl Changable for MyUnstreamable {
        fn apply(&mut self, event: Self::EventType) -> Self::EventType {
            match event {
                Created(id, name) => {
                    self.0 = *id.raw();
                    self.1 = name;
                    Deleted(id)
                }
                Deleted(id) => {
                    let name = mem::replace(&mut self.1, "");
                    Created(id, name)
                }
            }
        }
    }

    impl Historic for MyUnstreamable {
        type EventType = MyUnstreamableEvent;
    }

    struct MyStreamable;

    #[derive(Debug, Eq, PartialEq)]
    enum MyUnstreamableEvent {
        Created(Id<MyUnstreamable>, &'static str),
        Deleted(Id<MyUnstreamable>),
    }

    #[derive(Debug, Eq, PartialEq)]
    enum MyEvent {
        Captured(MyContext),
    }

    #[derive(Debug, Eq, PartialEq, Clone)]
    struct MyContext {
        name: &'static str,
    }

    impl StreamableInContext<MyContext> for MyStreamable {
        fn stream_in_context_to<S>(
            &mut self,
            context: &mut MyContext,
            stream: &mut S,
        ) -> Result<usize, Box<dyn Error>>
        where
            S: Stream<Self::EventType>,
        {
            stream.stream(vec![Captured(MyContext { name: context.name })])
        }
    }

    impl Historic for MyStreamable {
        type EventType = MyEvent;
    }
}
