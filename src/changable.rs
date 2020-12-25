use crate::change_abs::AppliedChange;

pub trait Changable: Sized {
    type EventType;

    fn apply(&mut self, event: Self::EventType) -> Self::EventType;

    fn applied<C>(&mut self, e: Self::EventType) -> C
    where
        C: AppliedChange<Self::EventType>,
    {
        C::from_application(e, |e| self.apply(e))
    }
}
