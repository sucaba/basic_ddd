use super::{BChange, BChanges};
use std::fmt::Debug;
use std::iter;
use std::ops;
use std::slice;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record<T> {
    inner: Vec<T>,
    redos: Vec<T>,
}

impl<T> Record<T> {
    pub fn new() -> Self {
        Record {
            inner: Vec::new(),
            redos: Vec::new(),
        }
    }

    pub fn history_len(&self) -> usize {
        self.inner.len()
    }

    pub fn reverse(&mut self) {
        self.inner.reverse();
    }

    pub fn iter(&self) -> slice::Iter<'_, T> {
        self.inner.iter()
    }

    pub fn take_after(&mut self, pos: usize) -> impl Iterator<Item = T> + '_ {
        self.inner.drain(pos..)
    }

    pub fn drain<R>(&mut self, range: R) -> impl Iterator<Item = T> + '_
    where
        R: ops::RangeBounds<usize>,
    {
        self.inner.drain(range)
    }

    pub fn push(&mut self, entry: T) {
        self.inner.push(entry)
    }

    pub fn push_redo(&mut self, entry: T) {
        self.redos.push(entry)
    }

    pub fn undo(&mut self) -> Option<T> {
        self.inner.pop()
    }

    pub fn redo(&mut self) -> Option<T> {
        self.redos.pop()
    }

    pub fn map<F, O>(self, f: F) -> Record<O>
    where
        F: Fn(T) -> O,
    {
        self.into_iter().map(f).collect()
    }
}

impl<T> Default for Record<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> iter::Extend<T> for Record<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.inner.extend(iter)
    }
}

impl<T, I> ops::Index<I> for Record<T>
where
    I: slice::SliceIndex<[T]>,
{
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.inner.index(index)
    }
}

impl<T> Into<Vec<T>> for Record<T> {
    fn into(self) -> Vec<T> {
        self.inner.into_iter().collect()
    }
}

impl<T> iter::FromIterator<T> for Record<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self {
            inner: iter.into_iter().collect(),
            redos: Vec::new(),
        }
    }
}

impl<T> From<BChanges<T>> for Record<BChange<T>> {
    fn from(src: BChanges<T>) -> Self {
        src.into_iter().collect()
    }
}

impl<T> iter::IntoIterator for Record<T> {
    type Item = T;
    type IntoIter = <Vec<T> as iter::IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use TestEvent::*;

    #[non_exhaustive]
    enum TestEvent {
        Incremented(usize),
        Decremented(usize),
    }

    #[test]
    fn should_extend_history() {
        let mut sut = Record::new();
        sut.extend(&[BChange {
            redo: Incremented(1),
            undo: Decremented(1),
        }]);

        assert_eq!(sut.history_len(), 1);
    }
}
