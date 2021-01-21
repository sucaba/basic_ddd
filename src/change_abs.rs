use crate::changes::{FullChange, FullChanges};

pub trait AppliedChange<T> {
    fn from_application<F>(redo: T, make_undo: F) -> Self
    where
        F: FnOnce(T) -> T;

    fn from_application_of_many<I, F>(redos: I, make_undo: F) -> Self
    where
        I: IntoIterator<Item = T>,
        F: FnMut(T) -> T;
}

pub trait NoopChange {
    fn noop() -> Self;
}

impl<T> AppliedChange<T> for FullChanges<T>
where
    T: Clone,
{
    fn from_application<F>(redo: T, make_undo: F) -> Self
    where
        F: FnOnce(T) -> T,
    {
        let undo = make_undo(redo.clone());
        FullChanges::only(FullChange::new(redo, undo))
    }

    fn from_application_of_many<I, F>(redos: I, mut make_undo: F) -> Self
    where
        I: IntoIterator<Item = T>,
        F: FnMut(T) -> T,
    {
        redos
            .into_iter()
            .map(|redo| {
                let undo = make_undo(redo.clone());
                FullChange::new(redo, undo)
            })
            .collect()
    }
}

impl<T> NoopChange for FullChanges<T> {
    fn noop() -> Self {
        Self::new()
    }
}
