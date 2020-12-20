use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};

use crate::changes::*;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::mem;
use std::rc::Rc;

pub type BasicChange<T> = BChange<<T as Changable>::EventType>;
pub type Changes<T> = BChanges<<T as Changable>::EventType>;
pub type UndoManager<T> = Record<BChange<<T as Changable>::EventType>>;

pub fn applied<S, T>(redo: T, subj: &mut S) -> BChanges<T>
where
    S: Changable<EventType = T>,
    T: Clone,
{
    BChanges::only(applied_one(redo, subj))
}

pub fn applied_one<S, T>(redo: T, subj: &mut S) -> BChange<T>
where
    S: Changable<EventType = T>,
    T: Clone,
{
    BChange::applied(redo, |e| subj.apply(e.clone()))
}

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
        Self { id }
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

pub trait Undoable: Changable + Sized {
    fn undomanager_mut(&mut self) -> &mut UndoManager<Self>;

    fn begin_changes(&mut self) -> Atomic<'_, Self> {
        let check_point = self.undomanager_mut().history_len();
        Atomic {
            subj: self,
            check_point,
        }
    }

    fn undo(&mut self) -> bool
    where
        Self::EventType: Clone,
    {
        if let Some(c) = self.undomanager_mut().undo() {
            let change = applied_one(c.undo, self);
            self.undomanager_mut().push_redo(change);
            true
        } else {
            false
        }
    }

    fn redo(&mut self) -> bool
    where
        Self::EventType: Clone,
    {
        if let Some(c) = self.undomanager_mut().redo() {
            let change = applied_one(c.undo, self);
            self.undomanager_mut().push_undo(change);
            true
        } else {
            false
        }
    }

    fn undo_all(&mut self)
    where
        Self::EventType: Clone,
    {
        while self.undo() {}
    }

    fn redo_n(&mut self, n: usize)
    where
        Self::EventType: Clone,
    {
        for _ in 0..n {
            self.redo();
        }
    }

    fn forget_changes(&mut self) {
        mem::take(self.undomanager_mut());
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

impl<T: Undoable> Streamable for T
where
    Self::EventType: Clone,
{
    fn stream_to<S>(&mut self, stream: &mut S)
    where
        S: Stream<Self::EventType>,
        Self::EventType: Clone,
    {
        let changes = UndoRedoStreamingStrategy::streamable_events(self);
        stream.stream(changes);
    }
}

pub struct UndoRedoStreamingStrategy;

impl UndoRedoStreamingStrategy {
    fn streamable_events<TEvent, U>(undoable: &mut U) -> Vec<TEvent>
    where
        U: Undoable<EventType = TEvent>,
        TEvent: Clone,
    {
        let count = undoable.undomanager_mut().history_len();
        undoable.undo_all();
        let result = undoable
            .undomanager_mut()
            .iter_n_redos(count)
            .map(|c| c.undo.clone())
            .collect();
        undoable.redo_n(count);
        result
    }
}

pub trait Unstreamable: Changable + Default + Sized {
    fn load<'a, I>(events: I) -> crate::result::Result<Self>
    where
        I: IntoIterator<Item = Self::EventType>;
}

pub struct Atomic<'a, T: Undoable> {
    subj: &'a mut T,
    check_point: usize,
}

impl<'a, T: Undoable> Atomic<'a, T> {
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
        self.subj.undomanager_mut().extend(changes);
        Ok(())
    }

    pub fn mutate_inner<InnerEvent, M, B, E>(
        &mut self,
        change_inner: M,
        bubble_up: B,
    ) -> Result<(), E>
    where
        M: FnOnce(&mut T) -> Result<BChanges<InnerEvent>, E>,
        B: Clone + Fn(InnerEvent) -> T::EventType,
    {
        let inner_changes = change_inner(self.subj)?;
        let changes = inner_changes.bubble_up(bubble_up);
        self.subj.undomanager_mut().extend(changes);

        Ok(())
    }

    pub fn commit(self) {
        mem::forget(self)
    }
}

impl<'a, T: Undoable> Drop for Atomic<'a, T> {
    fn drop(&mut self) {
        let mut to_compensate: Vec<_> = self
            .subj
            .undomanager_mut()
            .take_after(self.check_point)
            .collect();
        to_compensate.reverse();
        for BChange { undo, .. } in to_compensate {
            self.subj.apply(undo);
        }
    }
}

impl<T, TEvent> Unstreamable for T
where
    T: Sized + Default + Changable<EventType = TEvent>,
    T::EventType: Debug,
{
    fn load<I>(events: I) -> crate::result::Result<Self>
    where
        I: IntoIterator<Item = Self::EventType>,
    {
        let mut result = Self::default();
        for e in events {
            // println!("{}: {:#?}", i, e);
            let _on_undoable_change = result.apply(e);
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

            let undo = self.apply(Started);
            self.changes.push_undo(BChange {
                redo: Started,
                undo,
            });
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
                _ => panic!("not supported {:#?}", (self.state, event)),
            }
        }
    }

    impl Undoable for TestEntry {
        fn undomanager_mut(&mut self) -> &mut UndoManager<Self> {
            &mut self.changes
        }
    }

    fn given_stopped() -> TestEntry {
        let sut = TestEntry {
            state: Stopped,
            changes: UndoManager::<TestEntry>::new(),
        };

        assert_eq!(Stopped, sut.state);
        sut
    }

    #[test]
    fn should_apply_change() {
        let mut sut = given_stopped();

        sut.start().unwrap();

        assert_eq!(sut.state, Started);
    }

    #[test]
    fn should_undo_change() {
        let mut sut = given_stopped();

        sut.start().unwrap();
        sut.undo();

        assert_eq!(sut.state, Stopped);
    }

    #[test]
    fn should_redo_change() {
        let mut sut = given_stopped();

        sut.start().unwrap();
        sut.undo();

        sut.redo();

        assert_eq!(sut.state, Started);
    }

    #[test]
    fn should_implicitly_rollback_changes() {
        let mut sut = given_stopped();

        assert_eq!(
            sut.double_start(),
            Err("Already started".to_string().into())
        );

        assert_eq!(Stopped, sut.state);

        let changes = sut.take_changes();
        assert_eq!(Vec::<TestEvent>::new(), changes);
    }
}
