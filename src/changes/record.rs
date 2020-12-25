use super::FullChange;
use std::fmt::Debug;
use std::iter;
use std::ops;
use std::slice;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record<T> {
    undos: Vec<FullChange<T>>,
    redos: Vec<FullChange<T>>,
}

impl<T> Record<T> {
    pub fn new() -> Self {
        Record {
            undos: Vec::new(),
            redos: Vec::new(),
        }
    }

    pub fn history_len(&self) -> usize {
        self.undos.len()
    }

    pub fn take_after(&mut self, pos: usize) -> impl Iterator<Item = FullChange<T>> + '_ {
        self.undos.drain(pos..)
    }

    pub fn make_applied<F>(entry: T, make_undo: F) -> FullChange<T>
    where
        F: FnOnce(T) -> T,
        T: Clone,
    {
        let undo = make_undo(entry.clone());
        FullChange::new(entry, undo)
    }

    pub fn push_undo(&mut self, entry: FullChange<T>) {
        self.undos.push(entry)
    }

    pub fn push_redo(&mut self, entry: FullChange<T>) {
        self.redos.push(entry)
    }

    pub fn pop_undo(&mut self) -> Option<FullChange<T>> {
        self.undos.pop()
    }

    pub fn pop_redo(&mut self) -> Option<FullChange<T>> {
        self.redos.pop()
    }

    pub fn iter_last_redos(
        &mut self,
        count: usize,
    ) -> impl '_ + DoubleEndedIterator<Item = &FullChange<T>> {
        let len = self.redos.len();
        self.redos[(len - count)..len].iter()
    }

    pub fn iter_undos(&mut self) -> impl '_ + DoubleEndedIterator<Item = &FullChange<T>> {
        self.undos.iter()
    }
}

impl<T> Default for Record<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> iter::Extend<FullChange<T>> for Record<T> {
    fn extend<I: IntoIterator<Item = FullChange<T>>>(&mut self, iter: I) {
        self.undos.extend(iter)
    }
}

impl<T, I> ops::Index<I> for Record<T>
where
    I: slice::SliceIndex<[FullChange<T>]>,
{
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.undos.index(index)
    }
}

impl<T> Into<Vec<FullChange<T>>> for Record<T> {
    fn into(self) -> Vec<FullChange<T>> {
        self.undos.into_iter().collect()
    }
}

impl<T> iter::FromIterator<FullChange<T>> for Record<T> {
    fn from_iter<I: IntoIterator<Item = FullChange<T>>>(iter: I) -> Self {
        Self {
            undos: iter.into_iter().collect(),
            redos: Vec::new(),
        }
    }
}

impl<T> iter::IntoIterator for Record<T> {
    type Item = FullChange<T>;
    type IntoIter = <Vec<FullChange<T>> as iter::IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.undos.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    struct TestEvent;

    #[test]
    fn should_extend_history() {
        let mut sut = Record::new();
        sut.extend(vec![FullChange::new(TestEvent, TestEvent)]);

        assert_eq!(sut.history_len(), 1);
    }
}
