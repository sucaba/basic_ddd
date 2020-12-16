use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};

use crate::changes::*;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::mem;
use std::ops;
use std::rc::Rc;

pub type BasicChange<T> = BChange<<T as Changable>::EventType>;
pub type Changes<T> = BChanges<<T as Changable>::EventType>;

pub fn applied<S, T>(redo: T, subj: &mut S) -> BChanges<T>
where
    S: Changable<EventType = T>,
    T: Clone,
{
    let undo = subj.apply(redo.clone());
    BChanges::only(BChange::<T> { redo, undo })
}

pub struct UndoManager<T: Changable> {
    inner: Record<BasicChange<T>>,
}

impl<T: Changable> fmt::Debug for UndoManager<T>
where
    T::EventType: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("UndoManager")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<T: Changable> UndoManager<T> {
    pub fn new() -> Self {
        UndoManager {
            inner: Record::new(),
        }
    }

    pub fn history_len(&self) -> usize {
        self.inner.history_len()
    }

    pub fn reverse(&mut self) {
        self.inner.reverse();
    }

    pub fn iter(&self) -> std::slice::Iter<'_, BasicChange<T>> {
        self.inner.iter()
    }

    pub fn take_after(&mut self, pos: usize) -> impl Iterator<Item = BasicChange<T>> + '_ {
        self.inner.take_after(pos)
    }

    pub fn drain<'a, R>(&'a mut self, range: R) -> impl Iterator<Item = BasicChange<T>> + 'a
    where
        R: ops::RangeBounds<usize> + 'a,
    {
        self.inner.drain(range)
    }

    pub fn push(&mut self, redo: T::EventType, undo: T::EventType) {
        self.inner.push(BasicChange::<T> { redo, undo })
    }

    pub fn append<I: IntoIterator<Item = BasicChange<T>>>(&mut self, iter: I) {
        self.inner.extend(iter)
    }
}

impl<T: Changable> PartialEq for UndoManager<T>
where
    T::EventType: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&self.inner, &other.inner)
    }
}

impl<T: Changable> From<Changes<T>> for UndoManager<T> {
    fn from(changes: Changes<T>) -> Self {
        Self {
            inner: changes.into_iter().collect(),
        }
    }
}

impl<T: Changable> Eq for UndoManager<T> where T::EventType: Eq {}

pub trait Identifiable: Sized {
    type IdType: Eq;

    fn id(&self) -> Id<Self>;
}

impl<T: Changable> Default for UndoManager<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Changable> Clone for UndoManager<T>
where
    T::EventType: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: Changable> std::iter::FromIterator<BasicChange<T>> for UndoManager<T> {
    fn from_iter<I: IntoIterator<Item = BasicChange<T>>>(iter: I) -> Self {
        Self {
            inner: iter.into_iter().collect(),
        }
    }
}

impl<T: Changable> std::iter::IntoIterator for UndoManager<T> {
    type Item = BasicChange<T>;
    type IntoIter = <Vec<BasicChange<T>> as std::iter::IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
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

pub trait Undoable: Changable + Sized {
    fn undomanager_mut(&mut self) -> &mut UndoManager<Self>;

    fn begin_changes(&mut self) -> Atomic<'_, Self> {
        let check_point = self.undomanager_mut().history_len();
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

impl<T: Undoable> Streamable for T {
    fn stream_to<S>(&mut self, stream: &mut S)
    where
        S: Stream<Self::EventType>,
    {
        let changes = mem::take(self.undomanager_mut());
        stream.stream(changes.into_iter().map(BChange::take_redo));
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
        self.subj.undomanager_mut().append(changes);
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
        self.subj.undomanager_mut().append(changes);

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

    impl Undoable for TestEntry {
        fn undomanager_mut(&mut self) -> &mut UndoManager<Self> {
            &mut self.changes
        }
    }

    #[test]
    fn should_implicitly_rollback_changes() {
        let mut sut = TestEntry {
            state: Stopped,
            changes: vec![BChange {
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
