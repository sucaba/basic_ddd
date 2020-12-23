use super::changable::Changable;
use crate::undoable::{UndoManager, Undoable};
use std::fmt::Debug;

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
    {
        let mut strategy = UndoRedoStreamingStrategy::new(self);
        // TODO: Pass event references and not values
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
        self.um.iter_last_redos(self.count).rev().map(|c| c.undo())
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
