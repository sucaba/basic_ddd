use crate::changes::{FullChange, FullChanges};

pub trait AppliedChange<T> {
    fn from_application<F>(redo: T, make_undo: F) -> Self
    where
        F: FnOnce(T) -> T;
}

impl<T> AppliedChange<T> for FullChange<T>
where
    T: Clone,
{
    fn from_application<F>(redo: T, make_undo: F) -> Self
    where
        F: FnOnce(T) -> T,
    {
        let undo = make_undo(redo.clone());
        FullChange::new(redo, undo)
    }
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
}
