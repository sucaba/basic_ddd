use std::fmt::Debug;
use std::iter;
use std::ops;
use std::slice;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record<T> {
    undos: Vec<T>,
    redos: Vec<T>,
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

    pub fn take_after(&mut self, pos: usize) -> impl Iterator<Item = T> + '_ {
        self.undos.drain(pos..)
    }

    pub fn push_undo(&mut self, entry: T) {
        self.undos.push(entry)
    }

    pub fn push_redo(&mut self, entry: T) {
        self.redos.push(entry)
    }

    pub fn pop_undo(&mut self) -> Option<T> {
        self.undos.pop()
    }

    pub fn pop_redo(&mut self) -> Option<T> {
        self.redos.pop()
    }

    pub fn iter_last_redos(&mut self, count: usize) -> impl '_ + DoubleEndedIterator<Item = &T> {
        let len = self.redos.len();
        self.redos[(len - count)..len].iter()
    }

    pub fn iter_undos(&mut self) -> impl '_ + DoubleEndedIterator<Item = &T> {
        self.undos.iter()
    }
}

impl<T> Default for Record<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> iter::Extend<T> for Record<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.undos.extend(iter)
    }
}

impl<T, I> ops::Index<I> for Record<T>
where
    I: slice::SliceIndex<[T]>,
{
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.undos.index(index)
    }
}

impl<T> Into<Vec<T>> for Record<T> {
    fn into(self) -> Vec<T> {
        self.undos.into_iter().collect()
    }
}

impl<T> iter::FromIterator<T> for Record<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self {
            undos: iter.into_iter().collect(),
            redos: Vec::new(),
        }
    }
}

impl<T> iter::IntoIterator for Record<T> {
    type Item = T;
    type IntoIter = <Vec<T> as iter::IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.undos.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    struct TestChange;

    #[test]
    fn should_extend_history() {
        let mut sut = Record::new();
        sut.extend(vec![TestChange]);

        assert_eq!(sut.history_len(), 1);
    }
}
