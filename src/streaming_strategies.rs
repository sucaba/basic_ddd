use crate::changable::Changable;
use crate::streamable::Streamable;
use crate::streaming::Stream;
use crate::undoable::{UndoManager, Undoable};

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
        self.um.iter_future_history(self.count).rev()
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

impl<'a, U: Undoable> Changable for UndoRedoStreamingStrategy<'a, U>
where
    U::EventType: Clone,
{
    type EventType = U::EventType;

    fn apply(&mut self, _event: Self::EventType) -> Self::EventType {
        unimplemented!("Cannot modify through the strategy")
    }
}

impl<'a, U: Undoable> Streamable for UndoRedoStreamingStrategy<'a, U>
where
    U::EventType: Clone,
{
    fn stream_to<S>(&mut self, stream: &mut S)
    where
        S: Stream<U::EventType>,
    {
        stream.stream(self.events().into_iter().cloned());
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
        self.um.iter_history()
    }
}

impl<'a, U: Undoable> Streamable for CloneRedoStreamingStrategy<'a, U>
where
    U::EventType: Clone,
{
    fn stream_to<S>(&mut self, stream: &mut S)
    where
        S: Stream<U::EventType>,
    {
        stream.stream(self.events().into_iter().cloned());
    }
}

impl<'a, U: Undoable> Changable for CloneRedoStreamingStrategy<'a, U>
where
    U::EventType: Clone,
{
    type EventType = U::EventType;

    fn apply(&mut self, _event: Self::EventType) -> Self::EventType {
        unimplemented!("Cannot modify through the strategy")
    }
}
