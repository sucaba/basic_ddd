use super::changable::Changable;
use crate::contextual::Contextual;
use crate::historic::Historic;
use crate::identifiable::{Id, Identifiable};
use crate::streaming::*;
use std::hash::Hash;
use std::{collections::HashMap, error::Error};

pub trait Streamable: Historic {
    fn stream_to<S>(&mut self, stream: &mut S) -> Result<(), Box<dyn Error>>
    where
        S: Stream<Self::EventType>;

    // TODO: Result<Vec<..>>
    fn take_changes(&mut self) -> Vec<Self::EventType> {
        let mut result = Vec::new();
        self.stream_to(&mut result).unwrap();
        result
    }
}

pub trait SupportsDeletion {
    fn is_deletion(&self) -> bool;
}

/*
impl<'a, T: Streamable> Streamable for &'a T {
    fn stream_to<S>(self, stream: &mut S) -> Result<(), Box<dyn Error>>
    where
        S: Stream<Self::EventType> {
        self.as_mut().stream_to(stream)
    }
}
*/

pub trait StreamableInContext<TCtx>: Historic {
    fn stream_in_context_to<S>(
        &mut self,
        context: &mut TCtx,
        stream: &mut S,
    ) -> Result<(), Box<dyn Error>>
    where
        S: Stream<Self::EventType>;

    fn take_changes_in_context(&mut self, context: &mut TCtx) -> Vec<Self::EventType> {
        let mut result = Vec::new();
        self.stream_in_context_to(context, &mut result).unwrap();
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
    fn stream_to<S>(&mut self, stream: &mut S) -> Result<(), Box<dyn Error>>
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

    fn load_many<I>(events: I) -> crate::result::Result<Vec<Self>>
    where
        Self: Identifiable,
        Self::EventType: SupportsDeletion,
        I: IntoIterator<Item = (Id<Self>, Self::EventType)>,
        Id<Self>: Hash;
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

    fn load_many<I>(events: I) -> crate::result::Result<Vec<Self>>
    where
        Self: Identifiable,
        Self::EventType: SupportsDeletion,
        I: IntoIterator<Item = (Id<Self>, Self::EventType)>,
        Id<Self>: Hash,
    {
        let mut map = HashMap::<Id<Self>, Self>::new();
        for (id, e) in events {
            if e.is_deletion() {
                let _ = map.remove_entry(&id);
            } else {
                let aggregate = map.entry(id).or_default();
                let _non_undoable_change = aggregate.apply(e);
            }
        }

        Ok(map.into_iter().map(|p| p.1).collect())
    }
}

#[cfg(test)]
mod tests {
    use std::mem;

    use super::*;
    use crate::contextual::InContext;
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
        let _ = sut.stream_to(&mut stream);

        assert_eq!(stream, vec![Captured(MyContext { name: "exotic" })]);
    }

    #[test]
    fn should_load_multiple() {
        let events = vec![
            (Id::new(42), Created(Id::new(42), "red")),
            (Id::new(13), Created(Id::new(13), "green")),
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
            (Id::new(42), Created(Id::new(42), "red")),
            (Id::new(13), Created(Id::new(13), "green")),
            (Id::new(42), Deleted(Id::new(42))),
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

    impl SupportsDeletion for MyUnstreamableEvent {
        fn is_deletion(&self) -> bool {
            matches!(self, MyUnstreamableEvent::Deleted(_))
        }
    }

    impl Changable for MyUnstreamable {
        fn apply(&mut self, event: Self::EventType) -> Self::EventType {
            match event {
                Created(id, name) => {
                    self.0 = *id.id();
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
        ) -> Result<(), Box<dyn Error>>
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
