use super::Change;
use std::fmt::Debug;
use std::iter;
use std::ops;
use std::slice;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record<T> {
    undos: Vec<Change<T>>,
    redos: Vec<Change<T>>,
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

    pub fn reverse(&mut self) {
        self.undos.reverse();
    }

    pub fn iter(&self) -> slice::Iter<'_, Change<T>> {
        self.undos.iter()
    }

    pub fn take_after(&mut self, pos: usize) -> impl Iterator<Item = Change<T>> + '_ {
        self.undos.drain(pos..)
    }

    pub fn drain<R>(&mut self, range: R) -> impl Iterator<Item = Change<T>> + '_
    where
        R: ops::RangeBounds<usize>,
    {
        self.undos.drain(range)
    }

    pub fn make_applied<F>(entry: T, make_undo: F) -> Change<T>
    where
        F: FnOnce(T) -> T,
        T: Clone,
    {
        let undo = make_undo(entry.clone());
        Change::new(entry, undo)
    }

    pub fn push_undo(&mut self, entry: Change<T>) {
        self.undos.push(entry)
    }

    pub fn push_redo(&mut self, entry: Change<T>) {
        self.redos.push(entry)
    }

    pub fn pop_undo(&mut self) -> Option<Change<T>> {
        self.undos.pop()
    }

    pub fn pop_redo(&mut self) -> Option<Change<T>> {
        self.redos.pop()
    }

    pub fn iter_last_redos(
        &mut self,
        count: usize,
    ) -> impl '_ + DoubleEndedIterator<Item = &Change<T>> {
        let len = self.redos.len();
        self.redos[(len - count)..len].iter()
    }

    pub fn iter_undos(&mut self) -> impl '_ + DoubleEndedIterator<Item = &Change<T>> {
        self.undos.iter()
    }
}

impl<T> Default for Record<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> iter::Extend<Change<T>> for Record<T> {
    fn extend<I: IntoIterator<Item = Change<T>>>(&mut self, iter: I) {
        self.undos.extend(iter)
    }
}

impl<T, I> ops::Index<I> for Record<T>
where
    I: slice::SliceIndex<[Change<T>]>,
{
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.undos.index(index)
    }
}

impl<T> Into<Vec<Change<T>>> for Record<T> {
    fn into(self) -> Vec<Change<T>> {
        self.undos.into_iter().collect()
    }
}

impl<T> iter::FromIterator<Change<T>> for Record<T> {
    fn from_iter<I: IntoIterator<Item = Change<T>>>(iter: I) -> Self {
        Self {
            undos: iter.into_iter().collect(),
            redos: Vec::new(),
        }
    }
}

impl<T> iter::IntoIterator for Record<T> {
    type Item = Change<T>;
    type IntoIter = <Vec<Change<T>> as iter::IntoIterator>::IntoIter;

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
        sut.extend(vec![Change::new(TestEvent, TestEvent)]);

        assert_eq!(sut.history_len(), 1);
    }
}
