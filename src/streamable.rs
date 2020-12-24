use super::changable::Changable;
use crate::streaming::*;
use crate::undoable::{UndoManager, Undoable};
use std::fmt::Debug;

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
        //let mut strategy = UndoRedoStreamingStrategy::new(self);
        let mut strategy = CloneRedoStreamingStrategy::new(self);
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
    pub fn new(undoable: &'a mut U) -> Self {
        let count = undoable.changes_mut().history_len();
        let mut um = undoable.undo_manager();
        um.undo_all();
        Self { um, count }
    }

    pub fn events(&mut self) -> impl IntoIterator<Item = &U::EventType> {
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

pub struct CloneRedoStreamingStrategy<'a, U: Undoable>
where
    U::EventType: Clone,
{
    um: UndoManager<'a, U>,
}

impl<'a, U: Undoable> CloneRedoStreamingStrategy<'a, U>
where
    U::EventType: Clone,
{
    pub fn new(undoable: &'a mut U) -> Self {
        Self {
            um: undoable.undo_manager(),
        }
    }

    pub fn events(&mut self) -> impl IntoIterator<Item = &U::EventType> {
        self.um.iter_undos().map(|c| c.redo())
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
