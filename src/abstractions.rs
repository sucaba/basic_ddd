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

    fn applied_one(&mut self, e: Self::EventType) -> Change<Self::EventType>
    where
        Self::EventType: Clone,
    {
        Change::applied(e, |e| self.apply(e))
    }

    fn applied(&mut self, e: Self::EventType) -> Changes<Self::EventType>
    where
        Self::EventType: Clone,
    {
        Changes::only(Change::applied(e, |e| self.apply(e)))
    }
}

pub trait Undoable: Changable + Sized {
    fn changes_mut(&mut self) -> &mut Record<Self::EventType>;

    fn begin_changes(&mut self) -> Atomic<'_, Self> {
        let check_point = self.changes_mut().history_len();
        Atomic {
            subj: self,
            check_point,
        }
    }

    fn undo_manager<'a>(&'a mut self) -> UndoManager<'a, Self> {
        UndoManager { subj: self }
    }

    fn forget_changes(&mut self) {
        mem::take(self.changes_mut());
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
        let mut strategy = UndoRedoStreamingStrategy::new(self);
        stream.stream(strategy.events().into_iter().cloned());
    }
}

pub struct UndoRedoStreamingStrategy<'a, U: Undoable>
where
    U::EventType: Clone,
{
    um: UndoManager<'a, U>,
    count: usize,
}

impl<'a, U: Undoable> UndoRedoStreamingStrategy<'a, U>
where
    U::EventType: Clone,
{
    fn new(undoable: &'a mut U) -> Self {
        let count = undoable.changes_mut().history_len();
        let mut um = undoable.undo_manager();
        um.undo_all();
        Self { um, count }
    }

    fn events(&mut self) -> impl IntoIterator<Item = &U::EventType> {
        self.um
            .iter_n_redos(self.count)
            .map(|c| &c.undo)
            .collect::<Vec<_>>()
    }
}

impl<'a, U: Undoable> Drop for UndoRedoStreamingStrategy<'a, U>
where
    U::EventType: Clone,
{
    fn drop(&mut self) {
        self.um.redo_n(self.count);
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
        F: FnOnce(&mut T) -> Result<Changes<T::EventType>, E>,
    {
        let changes = f(self.subj)?;
        self.subj.changes_mut().extend(changes);
        Ok(())
    }

    pub fn mutate_inner<InnerEvent, M, B, E>(
        &mut self,
        change_inner: M,
        bubble_up: B,
    ) -> Result<(), E>
    where
        M: FnOnce(&mut T) -> Result<Changes<InnerEvent>, E>,
        B: Clone + Fn(InnerEvent) -> T::EventType,
    {
        let inner_changes = change_inner(self.subj)?;
        let changes = inner_changes.bubble_up(bubble_up);
        self.subj.changes_mut().extend(changes);

        Ok(())
    }

    pub fn commit(self) {
        mem::forget(self)
    }
}

pub struct UndoManager<'a, T: Undoable> {
    subj: &'a mut T,
}

impl<'a, T: Undoable> UndoManager<'a, T> {
    fn changes_mut(&mut self) -> &mut Record<T::EventType> {
        self.subj.changes_mut()
    }

    pub fn undo(&mut self) -> bool
    where
        T::EventType: Clone,
    {
        if let Some(c) = self.changes_mut().undo() {
            let change = self.subj.applied_one(c.undo);
            self.changes_mut().push_redo(change);
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool
    where
        T::EventType: Clone,
    {
        if let Some(c) = self.changes_mut().redo() {
            let change = self.subj.applied_one(c.undo);
            self.changes_mut().push_undo(change);
            true
        } else {
            false
        }
    }

    pub fn undo_all(&mut self)
    where
        T::EventType: Clone,
    {
        while self.undo() {}
    }

    pub fn redo_n(&mut self, n: usize)
    where
        T::EventType: Clone,
    {
        for _ in 0..n {
            self.redo();
        }
    }

    pub fn forget_changes(&mut self) {
        mem::take(self.changes_mut());
    }

    fn iter_n_redos(&mut self, count: usize) -> impl '_ + Iterator<Item = &Change<T::EventType>>
    where
        T::EventType: Clone,
    {
        self.changes_mut().iter_n_redos(count)
    }
}

impl<'a, T: Undoable> Drop for Atomic<'a, T> {
    fn drop(&mut self) {
        let mut to_compensate: Vec<_> = self
            .subj
            .changes_mut()
            .take_after(self.check_point)
            .collect();
        to_compensate.reverse();
        for Change { undo, .. } in to_compensate {
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
        changes: Record<TestEvent>,
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
            self.changes.push_undo(Change {
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
        fn changes_mut(&mut self) -> &mut Record<Self::EventType> {
            &mut self.changes
        }
    }

    fn given_stopped() -> TestEntry {
        let sut = TestEntry {
            state: Stopped,
            changes: Record::new(),
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
        let mut ops = sut.undo_manager();
        ops.undo();

        assert_eq!(sut.state, Stopped);
    }

    #[test]
    fn should_redo_change() {
        let mut sut = given_stopped();

        sut.start().unwrap();

        let mut ops = sut.undo_manager();

        ops.undo();

        ops.redo();

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
