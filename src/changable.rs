use crate::change_abs::AppliedChange;
use crate::changes::{FullChange, FullChanges};

pub trait Changable: Sized {
    type EventType;

    fn apply(&mut self, event: Self::EventType) -> Self::EventType;

    fn applied(&mut self, e: Self::EventType) -> FullChanges<Self::EventType>
    where
        Self::EventType: Clone,
    {
        FullChanges::only(FullChange::applied(e, |e| self.apply(e)))
    }
}
