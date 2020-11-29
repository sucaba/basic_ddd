use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::mem;
use std::ops;
use std::rc::Rc;
use std::vec;

pub fn changes<T: Changable>(event: BasicChange<T>) -> Changes<T> {
    std::iter::once(event).collect()
}

pub struct BasicChange<T: Changable> {
    pub redo: T::EventType,
    pub undo: T::EventType,
}

impl<T: Changable> BasicChange<T> {
    pub fn take_redo(self) -> T::EventType {
        self.redo
    }

    pub fn take_undo(self) -> T::EventType {
        self.undo
    }

    pub fn convert<O, F>(self, f: F) -> BasicChange<O>
    where
        O: Changable,
        F: Fn(T::EventType) -> O::EventType,
    {
        BasicChange {
            redo: f(self.redo),
            undo: f(self.undo),
        }
    }
}

impl<T: Changable> Clone for BasicChange<T>
where
    T::EventType: Clone,
{
    fn clone(&self) -> Self {
        BasicChange {
            redo: self.redo.clone(),
            undo: self.undo.clone(),
        }
    }
}

impl<T> Debug for BasicChange<T>
where
    T: Changable,
    T::EventType: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BasicChange")
            .field("redo", &self.redo)
            .field("undo", &self.undo)
            .finish()
    }
}

impl<T> PartialEq for BasicChange<T>
where
    T: Changable,
    T::EventType: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.redo.eq(&other.redo) && self.undo.eq(&other.undo)
    }
}

pub struct Changes<T: Changable> {
    inner: SmallList<BasicChange<T>>,
}

pub struct AtomicChange<'a, T: Changable> {
    changes: &'a mut Changes<T>,
    size: usize,
}

impl<'a, T: Changable> AtomicChange<'a, T> {
    pub fn commit(self) {
        mem::forget(self)
    }
    pub fn rollback(mut self) {
        self.rollback_mut();
        mem::forget(self)
    }

    pub fn push(&mut self, redo: T::EventType, undo: T::EventType) {
        self.changes.push(redo, undo)
    }

    fn rollback_mut(&mut self) {
        self.changes.rollback_to(self.size);
    }
}

impl<'a, T: Changable> Drop for AtomicChange<'a, T> {
    fn drop(&mut self) {
        self.rollback_mut();
    }
}

impl<T: Changable> Changes<T> {
    pub fn new() -> Self {
        Changes {
            inner: SmallList::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, BasicChange<T>> {
        self.inner.iter()
    }

    pub fn take_after(&mut self, pos: usize) -> Self {
        Self {
            inner: self.inner.take_after(pos),
        }
    }

    pub fn drain<R>(&mut self, range: R) -> vec::Drain<'_, BasicChange<T>>
    where
        R: ops::RangeBounds<usize>,
    {
        self.inner.drain(range)
    }

    pub fn atomic<S, E, F>(&mut self, f: F) -> Result<S, E>
    where
        F: FnOnce(&mut AtomicChange<'_, T>) -> Result<S, E>,
    {
        let mut trx = self.begin();
        let result = f(&mut trx)?;
        trx.commit();
        Ok(result)
    }

    pub fn begin(&mut self) -> AtomicChange<'_, T> {
        AtomicChange {
            size: self.inner.len(),
            changes: self,
        }
    }

    fn rollback_to(&mut self, point: usize) {
        self.inner.truncate(point)
    }

    pub fn to(self, dest: &mut Self) {
        dest.extend_changes(self)
    }

    pub fn push(&mut self, redo: T::EventType, undo: T::EventType) {
        self.inner.push(BasicChange { redo, undo })
    }

    pub fn map<F, O: Changable>(self, f: F) -> Changes<O>
    where
        F: Fn(BasicChange<T>) -> BasicChange<O>,
    {
        self.into_iter().map(f).collect::<Changes<O>>()
    }

    pub fn bubble_up<O, F, A>(self, f: F, dest: &mut A)
    where
        O: Changable,
        F: Fn(BasicChange<T>) -> BasicChange<O>,
        A: ExtendChanges<O>,
    {
        dest.extend_changes(self.into_iter().map(f));
    }
}

impl<T, I> std::ops::Index<I> for Changes<T>
where
    T: Changable,
    I: std::slice::SliceIndex<[BasicChange<T>]>,
{
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.inner.index(index)
    }
}

pub trait ExtendChanges<O: Changable> {
    fn extend_changes<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = BasicChange<O>>;
}

impl<T: Changable> Default for Changes<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Changable> Clone for Changes<T>
where
    T::EventType: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: Changable> PartialEq for Changes<T>
where
    T::EventType: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&self.inner, &other.inner)
    }
}

impl<T: Changable> Eq for Changes<T> where T::EventType: Eq {}

impl<T: Changable> Debug for Changes<T>
where
    T::EventType: Debug,
{
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("Changes")
            .field("items", &self.inner)
            .finish()
    }
}

impl<T: Changable> ExtendChanges<T> for Changes<T> {
    fn extend_changes<I: IntoIterator<Item = BasicChange<T>>>(&mut self, iter: I) {
        self.inner.extend(iter)
    }
}

impl<T: Changable> ExtendChanges<T> for AtomicChange<'_, T> {
    fn extend_changes<I: IntoIterator<Item = BasicChange<T>>>(&mut self, iter: I) {
        self.changes.extend_changes(iter)
    }
}

impl<T: AtomicallyChangable> ExtendChanges<T> for Atomic<'_, T> {
    fn extend_changes<I: IntoIterator<Item = BasicChange<T>>>(&mut self, iter: I) {
        self.subj_mut().changes_mut().extend_changes(iter)
    }
}

impl<T: Changable> Into<Vec<BasicChange<T>>> for Changes<T> {
    fn into(self) -> Vec<BasicChange<T>> {
        self.inner.into_iter().collect()
    }
}

impl<T: Changable> std::iter::FromIterator<BasicChange<T>> for Changes<T> {
    fn from_iter<I: IntoIterator<Item = BasicChange<T>>>(iter: I) -> Self {
        Self {
            inner: iter.into_iter().collect(),
        }
    }
}

impl<T: Changable> std::iter::IntoIterator for Changes<T> {
    type Item = BasicChange<T>;
    type IntoIter = <Vec<BasicChange<T>> as std::iter::IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

struct SmallList<T> {
    inner: Vec<T>,
}

impl<T> Debug for SmallList<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_list().entries(&self.inner).finish()
    }
}

impl<T> SmallList<T> {
    pub fn new() -> Self {
        Self { inner: Vec::new() }
    }

    fn truncate(&mut self, size: usize) {
        self.inner.truncate(size)
    }

    pub fn take_after(&mut self, pos: usize) -> Self {
        Self {
            inner: self.inner.drain(pos..).collect(),
        }
    }

    pub fn drain<R>(&mut self, range: R) -> vec::Drain<'_, T>
    where
        R: ops::RangeBounds<usize>,
    {
        self.inner.drain(range)
    }

    pub fn push(&mut self, item: T) {
        self.inner.push(item)
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.inner.iter()
    }
}

impl<T, I> std::ops::Index<I> for SmallList<T>
where
    I: std::slice::SliceIndex<[T]>,
{
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.inner.index(index)
    }
}

impl<T: Clone> Clone for SmallList<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Extend<T> for SmallList<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.inner.extend(iter)
    }
}

impl<T> Into<Vec<T>> for SmallList<T> {
    fn into(self) -> Vec<T> {
        self.into_iter().collect()
    }
}

impl<T> std::iter::FromIterator<T> for SmallList<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self {
            inner: iter.into_iter().collect(),
        }
    }
}

impl<T: PartialEq> PartialEq for SmallList<T> {
    fn eq(&self, other: &Self) -> bool {
        return PartialEq::eq(&self.inner, &other.inner);
    }
}

impl<T: Eq> Eq for SmallList<T> {}

impl<T> std::iter::IntoIterator for SmallList<T> {
    type Item = T;
    type IntoIter = <Vec<T> as std::iter::IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
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

pub trait Changable {
    type EventType;

    fn apply(&mut self, event: &Self::EventType);

    #[inline]
    fn mutate(&mut self, redo: Self::EventType, undo: Self::EventType) -> Changes<Self>
    where
        Self: Sized,
    {
        self.apply(&redo);
        changes(BasicChange { redo, undo })
    }
}

pub struct Atomic<'a, T: AtomicallyChangable> {
    subj: &'a mut T,
    check_point: usize,
}

impl<'a, T: AtomicallyChangable> Atomic<'a, T> {
    pub fn subj(&self) -> &T {
        &self.subj
    }
    pub fn subj_mut(&mut self) -> &mut T {
        &mut self.subj
    }

    pub fn commit(self) {
        mem::forget(self)
    }
}

impl<'a, T: AtomicallyChangable> Drop for Atomic<'a, T> {
    fn drop(&mut self) {
        let to_compensate = self.subj.trim_changes(self.check_point);
        for BasicChange { undo, redo: _ } in to_compensate.iter() {
            self.subj.apply(&undo);
        }
    }
}

pub trait AtomicallyChangable: Changable + Sized {
    fn trim_changes(&mut self, check_point: usize) -> Changes<Self>;
    fn changes(&self) -> &Changes<Self>;
    fn changes_mut(&mut self) -> &mut Changes<Self>;

    fn begin<'a>(&'a mut self) -> Atomic<'a, Self> {
        let check_point = self.changes().len();
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
        I: IntoIterator<Item = &'a Self::EventType>,
        Self::EventType: 'static;
}

impl<T, TEvent> Unstreamable for T
where
    T: Sized + Default + Changable<EventType = TEvent>,
{
    fn load<'a, I>(events: I) -> crate::result::Result<Self>
    where
        I: IntoIterator<Item = &'a Self::EventType>,
        Self::EventType: 'static,
    {
        let mut result = Self::default();
        for e in events {
            result.apply(e);
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
        changes: Changes<TestEntry>,
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
        fn double_start(&mut self) -> Result<(), String> {
            let mut trx = self.begin();

            trx.subj_mut().start()?;
            // implictly rolledback after the following
            // failed call in a `trx.drop()`
            trx.subj_mut().start()?;
            Ok(())
        }

        fn start(&mut self) -> Result<(), String> {
            let was = self.state;
            if let Started = &self.state {
                return Err("Already started".into());
            }
            self.apply(&Started);
            self.changes.push(Started, was);
            Ok(())
        }
    }

    impl Changable for TestEntry {
        type EventType = TestEvent;

        fn apply(&mut self, event: &Self::EventType) {
            println!("applying {:#?}", event);
            match (self.state, event) {
                (Stopped, Started) | (Paused, Started) | (Started, Stopped) | (Started, Paused) => {
                    self.state = *event
                }
                _ => panic!("not supported"),
            }
        }
    }

    impl AtomicallyChangable for TestEntry {
        fn trim_changes(&mut self, check_point: usize) -> Changes<Self> {
            let result = self.changes.take_after(check_point);
            println!("take_after(checkpoint={})={:#?}", check_point, result);
            result
        }

        fn changes(&self) -> &Changes<Self> {
            &self.changes
        }

        fn changes_mut(&mut self) -> &mut Changes<Self> {
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

    /*
    #[test]
    fn should_commit_changes() {
        let mut changes = Changes::<TestEntry>::new();
        changes.push(Started,);
        let mut trx = changes.begin();
        trx.push(Stopped);
        trx.commit();

        let changes: Vec<_> = changes.into_iter().collect();
        assert_eq!(vec![Started, Stopped], changes);
    }

    #[test]
    fn should_commit_changes_using_atomic_fn() {
        let mut changes = Changes::<TestEntry>::new();
        changes.push(Started);
        let _ = changes.atomic(|trx| -> Result<(), ()> {
            trx.push(Stopped);
            Ok(())
        });

        let changes: Vec<_> = changes.into_iter().collect();
        assert_eq!(vec![Started, Stopped], changes);
    }

    #[test]
    fn should_implicitly_rollback_changes_using_atomic_fn() {
        let mut changes = Changes::<TestEntry>::new();
        changes.push(Started);
        let _ = changes.atomic(|trx| -> Result<(), ()> {
            trx.push(Stopped);
            Err(())
        });

        let changes: Vec<_> = changes.into_iter().collect();
        assert_eq!(vec![Started], changes);
    }

    #[test]
    fn should_implicitly_rollback_changes_old() {
        let mut changes = Changes::<TestEntry>::new();
        changes.push(Started);
        {
            let mut trx = changes.begin();
            trx.push(Stopped);
            // implictly rolled back here
        }

        let changes: Vec<_> = changes.into_iter().collect();
        assert_eq!(vec![Started], changes);
    }

    #[test]
    fn should_explicitly_rollback_changes_using_atomic_fn() {
        let mut changes = Changes::<TestEntry>::new();
        changes.push(Started);
        let _ = changes.atomic(|trx| -> Result<(), ()> {
            trx.push(Stopped);
            Err(())
        });

        let changes: Vec<_> = changes.into_iter().collect();
        assert_eq!(vec![Started], changes);
    }
    */

    #[test]
    fn should_implicitly_rollback_changes_new() {
        let mut sut = TestEntry {
            state: Stopped,
            changes: vec![BasicChange {
                redo: Stopped,
                undo: Stopped,
            }]
            .into_iter()
            .collect(),
        };

        assert_eq!(sut.double_start(), Err("Already started".into()));

        assert_eq!(Stopped, sut.state);

        let changes: Vec<_> = sut.take_changes();
        assert_eq!(vec![Stopped], changes);
    }
}
