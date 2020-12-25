use crate::changes::FullChange;

pub(crate) trait AppliedChange<T> {
    fn applied<F>(redo: T, make_undo: F) -> Self
    where
        F: FnOnce(T) -> T;
}

impl<T> AppliedChange<T> for FullChange<T>
where
    T: Clone,
{
    fn applied<F>(redo: T, make_undo: F) -> Self
    where
        F: FnOnce(T) -> T,
    {
        let undo = make_undo(redo.clone());
        FullChange::new(redo, undo)
    }
}
