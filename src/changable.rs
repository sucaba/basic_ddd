use crate::changes::{Change, Changes};

pub trait Changable: Sized {
    type EventType;

    fn apply(&mut self, event: Self::EventType) -> Self::EventType;

    fn applied(&mut self, e: Self::EventType) -> Changes<Self::EventType>
    where
        Self::EventType: Clone,
    {
        Changes::only(Change::applied(e, |e| self.apply(e)))
    }
}
