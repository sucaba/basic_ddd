use super::changable::Changable;
use crate::contextual::Contextual;
use crate::historic::Historic;
use crate::streaming::*;
use std::error::Error;

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
            let _on_undoable_change = result.apply(e);
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contextual::InContext;
    use pretty_assertions::assert_eq;
    use MyEvent::*;

    // Not really test for this module but rather a use case for implementing `Streamable` for
    // `Contextual`
    #[test]
    fn stream_in_context() {
        let entity_to_stream = MyEntity;
        let context = MyContext { name: "exotic" };

        let mut sut = entity_to_stream.in_context(context);
        let mut stream = Vec::new();
        let _ = sut.stream_to(&mut stream);

        assert_eq!(stream, vec![Captured(MyContext { name: "exotic" })]);
    }

    struct MyEntity;

    #[derive(Debug, Eq, PartialEq)]
    enum MyEvent {
        Captured(MyContext),
    }

    #[derive(Debug, Eq, PartialEq, Clone)]
    struct MyContext {
        name: &'static str,
    }

    impl StreamableInContext<MyContext> for MyEntity {
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

    impl Historic for MyEntity {
        type EventType = MyEvent;
    }
}
