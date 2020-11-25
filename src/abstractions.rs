use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::mem;
use std::rc::Rc;

pub fn changes<T: Changable>(event: T::EventType) -> Changes<T> {
    std::iter::once(event).collect()
}

pub struct Changes<T: Changable> {
    inner: SmallList<<T as Changable>::EventType>,
}

pub struct AtomicChange<'a, T: Changable> {
    owner: &'a mut Changes<T>,
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

    pub fn push(&mut self, item: T::EventType) {
        self.owner.push(item)
    }

    fn rollback_mut(&mut self) {
        self.owner.rollback_to(self.size);
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
            owner: self,
        }
    }

    fn rollback_to(&mut self, point: usize) {
        self.inner.truncate(point)
    }

    pub fn to(self, dest: &mut Self) {
        dest.extend_changes(self)
    }

    pub fn push(&mut self, event: T::EventType) {
        self.inner.push(event)
    }

    pub fn ascend<F, O: Changable>(self, f: F) -> Changes<O>
    where
        F: Fn(T::EventType) -> O::EventType,
    {
        self.into_iter().map(f).collect::<Changes<O>>()
    }

    /*
     * TODO: remove because immutable `f` causes issue
     */
    pub fn ascend_to<O, F, A>(self, f: F, dest: &mut A)
    where
        O: Changable,
        F: Fn(T::EventType) -> O::EventType,
        A: ExtendChanges<O>,
    {
        dest.extend_changes(self.into_iter().map(f));
    }
}

pub trait ExtendChanges<O: Changable> {
    fn extend_changes<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = O::EventType>;
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
    fn extend_changes<I: IntoIterator<Item = T::EventType>>(&mut self, iter: I) {
        self.inner.extend(iter)
    }
}

impl<T: Changable> ExtendChanges<T> for AtomicChange<'_, T> {
    fn extend_changes<I: IntoIterator<Item = T::EventType>>(&mut self, iter: I) {
        self.owner.extend_changes(iter)
    }
}

impl<T: Changable> Into<Vec<T::EventType>> for Changes<T> {
    fn into(self) -> Vec<T::EventType> {
        self.inner.into_iter().collect()
    }
}

impl<T: Changable> std::iter::FromIterator<T::EventType> for Changes<T> {
    fn from_iter<I: IntoIterator<Item = T::EventType>>(iter: I) -> Self {
        Self {
            inner: iter.into_iter().collect(),
        }
    }
}

impl<T: Changable> std::iter::IntoIterator for Changes<T> {
    type Item = T::EventType;
    type IntoIter = <Vec<T::EventType> as std::iter::IntoIterator>::IntoIter;

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

    pub fn push(&mut self, item: T) {
        self.inner.push(item)
    }

    pub fn len(&self) -> usize {
        self.inner.len()
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
    fn mutate(&mut self, e: Self::EventType) -> Changes<Self>
    where
        Self: Sized,
    {
        self.apply(&e);
        changes(e)
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

    #[derive(Debug, Eq, PartialEq)]
    struct TestEntry {
        state: TestEvent,
    }

    #[derive(Copy, Clone, Debug, Eq, PartialEq)]
    enum TestEvent {
        Started,
        Paused,
        Stopped,
    }

    impl Changable for TestEntry {
        type EventType = TestEvent;

        fn apply(&mut self, event: &Self::EventType) {
            use TestEvent::*;
            match (self.state, event) {
                (Stopped, Started) | (Paused, Started) | (Started, Stopped) | (Started, Paused) => {
                    self.state = *event
                }
                _ => panic!("not supported"),
            }
        }
    }

    #[test]
    fn should_commit_changes() {
        let mut changes = Changes::<TestEntry>::new();
        changes.push(TestEvent::Started);
        let mut trx = changes.begin();
        trx.push(TestEvent::Stopped);
        trx.commit();

        let changes: Vec<_> = changes.into_iter().collect();
        assert_eq!(vec![TestEvent::Started, TestEvent::Stopped], changes);
    }

    #[test]
    fn should_commit_changes_using_atomic_fn() {
        let mut changes = Changes::<TestEntry>::new();
        changes.push(TestEvent::Started);
        let _ = changes.atomic(|trx| -> Result<(), ()> {
            trx.push(TestEvent::Stopped);
            Ok(())
        });

        let changes: Vec<_> = changes.into_iter().collect();
        assert_eq!(vec![TestEvent::Started, TestEvent::Stopped], changes);
    }

    #[test]
    fn should_implicitly_rollback_changes_using_atomic_fn() {
        let mut changes = Changes::<TestEntry>::new();
        changes.push(TestEvent::Started);
        let _ = changes.atomic(|trx| -> Result<(), ()> {
            trx.push(TestEvent::Stopped);
            Err(())
        });

        let changes: Vec<_> = changes.into_iter().collect();
        assert_eq!(vec![TestEvent::Started], changes);
    }

    #[test]
    fn should_implicitly_rollback_changes() {
        let mut changes = Changes::<TestEntry>::new();
        changes.push(TestEvent::Started);
        {
            let mut trx = changes.begin();
            trx.push(TestEvent::Stopped);
            // implictly rolled back here
        }

        let changes: Vec<_> = changes.into_iter().collect();
        assert_eq!(vec![TestEvent::Started], changes);
    }

    #[test]
    fn should_explicitly_rollback_changes_using_atomic_fn() {
        let mut changes = Changes::<TestEntry>::new();
        changes.push(TestEvent::Started);
        let _ = changes.atomic(|trx| -> Result<(), ()> {
            trx.push(TestEvent::Stopped);
            Err(())
        });

        let changes: Vec<_> = changes.into_iter().collect();
        assert_eq!(vec![TestEvent::Started], changes);
    }
}
