use super::BChange;
use std::fmt::Debug;
use std::iter;
use std::ops;
use std::slice;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record<T> {
    undos: Vec<BChange<T>>,
    redos: Vec<BChange<T>>,
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

    pub fn iter(&self) -> slice::Iter<'_, BChange<T>> {
        self.undos.iter()
    }

    pub fn take_after(&mut self, pos: usize) -> impl Iterator<Item = BChange<T>> + '_ {
        self.undos.drain(pos..)
    }

    pub fn drain<R>(&mut self, range: R) -> impl Iterator<Item = BChange<T>> + '_
    where
        R: ops::RangeBounds<usize>,
    {
        self.undos.drain(range)
    }

    pub fn push_undo(&mut self, entry: BChange<T>) {
        self.undos.push(entry)
    }

    pub fn push_redo(&mut self, entry: BChange<T>) {
        self.redos.push(entry)
    }

    pub fn undo(&mut self) -> Option<BChange<T>> {
        self.undos.pop()
    }

    pub fn redo(&mut self) -> Option<BChange<T>> {
        self.redos.pop()
    }

    pub fn map<F, O>(self, f: F) -> Record<O>
    where
        F: Fn(BChange<T>) -> BChange<O>,
    {
        self.into_iter().map(f).collect()
    }

    pub fn iter_n_redos(&mut self, count: usize) -> impl '_ + Iterator<Item = &BChange<T>>
    where
        T: Clone,
    {
        let len = self.redos.len();
        self.redos[(len - count)..len].iter().rev()
    }
}

impl<T> Default for Record<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> iter::Extend<BChange<T>> for Record<T> {
    fn extend<I: IntoIterator<Item = BChange<T>>>(&mut self, iter: I) {
        self.undos.extend(iter)
    }
}

impl<T, I> ops::Index<I> for Record<T>
where
    I: slice::SliceIndex<[BChange<T>]>,
{
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.undos.index(index)
    }
}

impl<T> Into<Vec<BChange<T>>> for Record<T> {
    fn into(self) -> Vec<BChange<T>> {
        self.undos.into_iter().collect()
    }
}

impl<T> iter::FromIterator<BChange<T>> for Record<T> {
    fn from_iter<I: IntoIterator<Item = BChange<T>>>(iter: I) -> Self {
        Self {
            undos: iter.into_iter().collect(),
            redos: Vec::new(),
        }
    }
}

impl<T> iter::IntoIterator for Record<T> {
    type Item = BChange<T>;
    type IntoIter = <Vec<BChange<T>> as iter::IntoIterator>::IntoIter;

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
        sut.extend(vec![BChange {
            redo: TestEvent,
            undo: TestEvent,
        }]);

        assert_eq!(sut.history_len(), 1);
    }
}
