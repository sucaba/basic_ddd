use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};

use crate::changes::*;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::mem;

use std::rc::Rc;

pub trait Identifiable: Sized {
    type IdType: Eq;

    fn id(&self) -> Id<Self>;
}

pub trait GetId {
    type IdentifiableType: Identifiable;

    fn get_id(&self) -> Id<Self::IdentifiableType>;
}

impl<T> GetId for T
where
    T: Identifiable,
{
    type IdentifiableType = T;

    fn get_id(&self) -> Id<Self::IdentifiableType> {
        Identifiable::id(self)
    }
}

impl<T> GetId for Rc<T>
where
    T: GetId,
{
    type IdentifiableType = T::IdentifiableType;

    fn get_id(&self) -> Id<Self::IdentifiableType> {
        GetId::get_id(std::ops::Deref::deref(self))
    }
}

impl<T> GetId for Id<T>
where
    T: Identifiable,
    Self: Clone,
{
    type IdentifiableType = T;

    fn get_id(&self) -> Id<Self::IdentifiableType> {
        self.clone()
    }
}

pub trait Owned {
    type OwnerType: Identifiable;
}

pub struct Id<T: Identifiable> {
    id: T::IdType,
    marker: std::marker::PhantomData<T>,
}

impl<T: Identifiable> Hash for Id<T>
where
    T::IdType: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T: Identifiable> Id<T> {
    pub fn new(id: T::IdType) -> Self {
        Self {
            id,
            marker: Default::default(),
        }
    }

    pub fn id(&self) -> &T::IdType {
        &self.id
    }

    pub fn convert<U>(self) -> Id<U>
    where
        U: Identifiable<IdType = T::IdType>,
    {
        Id::new(self.id)
    }
}

impl<T: Identifiable> Copy for Id<T>
where
    Self: Clone,
    T::IdType: Copy,
{
}

impl<T: Identifiable> Clone for Id<T>
where
    T::IdType: Clone,
{
    fn clone(&self) -> Self {
        Id::new(self.id.clone())
    }
}

impl<T: Identifiable> Debug for Id<T>
where
    T::IdType: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.id.fmt(f)
    }
}

impl<T: Identifiable> Ord for Id<T>
where
    T::IdType: Ord,
{
    fn cmp(&self, y: &Id<T>) -> Ordering {
        Ord::cmp(&self.id, &y.id)
    }
}

impl<T: Identifiable> PartialOrd for Id<T>
where
    T::IdType: PartialOrd,
{
    fn partial_cmp(&self, y: &Id<T>) -> Option<Ordering> {
        self.id.partial_cmp(&y.id)
    }
}

impl<T: Identifiable> Eq for Id<T> {}

impl<T: Identifiable> PartialEq for Id<T> {
    fn eq(&self, y: &Id<T>) -> bool {
        self.id.eq(&y.id)
    }
}

pub enum EventMergeResult {
    Combined,
    Annihilated,
}

pub trait Changable: Sized {
    type EventType;

    fn apply(&mut self, event: Self::EventType) -> Self::EventType;
}

pub trait AtomicallyChangable: Changable + Sized {
    fn undomanager_mut(&mut self) -> &mut UndoManager<Self>;

    fn begin_changes<'a>(&'a mut self) -> Atomic<'a, Self> {
        let check_point = self.undomanager_mut().len();
        Atomic {
            subj: self,
            check_point,
        }
    }
}

pub trait Streamable: Changable {
    fn stream_to<S>(&mut self, stream: &mut S)
    where
        S: Stream<Self::EventType>;

    fn take_changes(&mut self) -> Vec<Self::EventType> {
        let mut result = Vec::new();
        self.stream_to(&mut result);
        result
    }
}

pub trait Unstreamable: Changable + Default + Sized {
    fn load<'a, I>(events: I) -> crate::result::Result<Self>
    where
        I: IntoIterator<Item = Self::EventType>;
}

pub struct Atomic<'a, T: AtomicallyChangable> {
    subj: &'a mut T,
    check_point: usize,
}

impl<'a, T: AtomicallyChangable> Atomic<'a, T> {
    pub fn invoke<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        f(self.subj)
    }

    pub fn mutate<F, E>(&mut self, f: F) -> Result<(), E>
    where
        F: FnOnce(&mut T) -> Result<Changes<T>, E>,
    {
        let changes = f(self.subj)?;
        self.subj.undomanager_mut().extend_changes(changes);
        Ok(())
    }

    pub fn mutate_inner<Inner, M, B, E>(&mut self, change_inner: M, bubble_up: B) -> Result<(), E>
    where
        Inner: Changable,
        M: FnOnce(&mut T) -> Result<Changes<Inner>, E>,
        B: Clone + Fn(Inner::EventType) -> T::EventType,
    {
        let inner_changes = change_inner(self.subj)?;
        let changes = inner_changes.bubble_up(bubble_up);
        self.subj.undomanager_mut().extend_changes(changes);

        Ok(())
    }

    pub fn commit(self) {
        mem::forget(self)
    }
}

impl<'a, T: AtomicallyChangable> Drop for Atomic<'a, T> {
    fn drop(&mut self) {
        let mut to_compensate = self.subj.undomanager_mut().take_after(self.check_point);
        to_compensate.reverse();
        for BasicChange { undo, .. } in to_compensate {
            self.subj.apply(undo);
        }
    }
}

impl<T, TEvent> Unstreamable for T
where
    T: Sized + Default + Changable<EventType = TEvent>,
{
    fn load<I>(events: I) -> crate::result::Result<Self>
    where
        I: IntoIterator<Item = Self::EventType>,
    {
        let mut result = Self::default();
        for e in events {
            let _ignored_change = result.apply(e);
        }

        Ok(result)
    }
}

pub trait Stream<TEvent>: Sized {
    fn stream<I>(&mut self, events: I)
    where
        I: IntoIterator<Item = TEvent>;
}

impl<S, TEvent> Stream<TEvent> for &mut S
where
    S: Stream<TEvent>,
{
    fn stream<I>(&mut self, events: I)
    where
        I: IntoIterator<Item = TEvent>,
    {
        (*self).stream(events);
    }
}

impl<TEvent> Stream<TEvent> for Vec<TEvent> {
    fn stream<I>(&mut self, events: I)
    where
        I: IntoIterator<Item = TEvent>,
    {
        self.extend(events);
    }
}

pub struct StreamAdapter<TInner, F>(TInner, F);

impl<TInner, F> StreamAdapter<TInner, F> {
    pub fn new(original: TInner, f: F) -> Self {
        Self(original, f)
    }
}

impl<TInnerEvent, TEvent, TInner, F> Stream<TEvent> for StreamAdapter<TInner, F>
where
    TInner: Stream<TInnerEvent>,
    F: Fn(TEvent) -> TInnerEvent,
{
    fn stream<I>(&mut self, events: I)
    where
        I: IntoIterator<Item = TEvent>,
    {
        self.0.stream(events.into_iter().map(&self.1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use TestEvent::*;

    #[derive(Debug, Eq, PartialEq)]
    struct TestEntry {
        state: TestEvent,
        changes: UndoManager<TestEntry>,
    }

    #[derive(Copy, Clone, Debug, Eq, PartialEq)]
    enum TestEvent {
        Stopped,
        Started,
        Paused,
    }

    impl TestEntry {
        /// Example of atomic operation which fails in the middle
        /// and causes compensation logic to restore state before
        /// the action start
        fn double_start(&mut self) -> crate::result::Result<()> {
            let mut trx = self.begin_changes();

            trx.invoke(Self::start)?;
            trx.invoke(Self::start)?; // fail and rollback both starts

            trx.commit();
            Ok(())
        }

        fn start(&mut self) -> Result<(), String> {
            self.validate_not_started()?;

            let was = self.state;
            self.apply(Started);
            self.changes.push(Started, was);
            Ok(())
        }

        fn validate_not_started(&self) -> Result<(), String> {
            if let Started = &self.state {
                return Err("Already started".into());
            }
            Ok(())
        }
    }

    impl Changable for TestEntry {
        type EventType = TestEvent;

        fn apply(&mut self, event: Self::EventType) -> Self::EventType {
            match (self.state, event) {
                (Stopped, Started) | (Paused, Started) | (Started, Stopped) | (Started, Paused) => {
                    let undo = self.state;
                    self.state = event;
                    undo
                }
                _ => panic!("not supported"),
            }
        }
    }

    impl AtomicallyChangable for TestEntry {
        fn undomanager_mut(&mut self) -> &mut UndoManager<Self> {
            &mut self.changes
        }
    }

    impl Streamable for TestEntry {
        fn stream_to<S>(&mut self, stream: &mut S)
        where
            S: Stream<Self::EventType>,
        {
            stream.stream(
                self.changes
                    .drain(..)
                    .map(|BasicChange { redo, undo: _ }| redo),
            )
        }
    }

    #[test]
    fn should_implicitly_rollback_changes() {
        let mut sut = TestEntry {
            state: Stopped,
            changes: vec![BasicChange {
                redo: Stopped,
                undo: Stopped,
            }]
            .into_iter()
            .collect(),
        };

        assert_eq!(
            sut.double_start(),
            Err("Already started".to_string().into())
        );

        assert_eq!(Stopped, sut.state);

        let changes: Vec<_> = sut.take_changes();
        assert_eq!(vec![Stopped], changes);
    }
}
