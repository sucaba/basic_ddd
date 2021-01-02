use super::changable::Changable;
use crate::streaming::*;
use std::error::Error;

pub trait Streamable: Changable {
    fn stream_to<S>(&mut self, stream: &mut S) -> Result<(), Box<dyn Error>>
    where
        S: Stream<Self::EventType>;

    fn take_changes(&mut self) -> Vec<Self::EventType> {
        let mut result = Vec::new();
        self.stream_to(&mut result).unwrap();
        result
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
    use crate::contextual::*;
    use pretty_assertions::assert_eq;
    use MyEvent::*;

    // Not really test for this module but rather a use case for implementing `Streamable` for
    // `Contextual`
    #[test]
    fn stream_in_context() {
        let entity_to_stream = MyEntity;
        let context = MyContext { name: "exotic" };

        let mut sut = Contextual {
            subject: entity_to_stream,
            context,
        };
        let mut stream = Vec::new();
        let _ = sut.stream_to(&mut stream);

        assert_eq!(stream, vec![ContextCaptured(MyContext { name: "exotic" })]);
    }

    struct MyEntity;

    #[derive(Debug, Eq, PartialEq)]
    enum MyEvent {
        Dummy,
        ContextCaptured(MyContext),
    }

    #[derive(Debug, Eq, PartialEq, Clone)]
    struct MyContext {
        name: &'static str,
    }

    impl Streamable for Contextual<MyEntity, MyContext> {
        fn stream_to<S>(&mut self, stream: &mut S) -> Result<(), Box<dyn Error>>
        where
            S: Stream<Self::EventType>,
        {
            stream.stream(vec![ContextCaptured(MyContext {
                name: self.context.name,
            })])
        }
    }

    impl Changable for Contextual<MyEntity, MyContext> {
        type EventType = MyEvent;

        fn apply(&mut self, _event: Self::EventType) -> Self::EventType {
            Dummy
        }
    }
}
