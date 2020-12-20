use std::fmt::Debug;
use std::vec;

#[derive(Clone, Default, Eq, PartialEq, Debug)]
pub struct SmallList<T> {
    inner: Vec<T>,
}

impl<T> SmallList<T> {
    pub fn new() -> Self {
        Self { inner: Vec::new() }
    }

    pub fn once(item: T) -> Self {
        Self { inner: vec![item] }
    }

    pub fn take_after(&mut self, pos: usize) -> Self {
        Self {
            inner: self.inner.drain(pos..).collect(),
        }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.inner.iter()
    }
}

impl<T, I> std::ops::Index<I> for SmallList<T>
where
    I: std::slice::SliceIndex<[T]>,
{
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.inner.index(index)
    }
}

impl<T> Extend<T> for SmallList<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.inner.extend(iter)
    }
}

impl<T> Into<Vec<T>> for SmallList<T> {
    fn into(self) -> Vec<T> {
        self.into_iter().collect()
    }
}

impl<T> std::iter::FromIterator<T> for SmallList<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self {
            inner: iter.into_iter().collect(),
        }
    }
}

impl<T> std::iter::IntoIterator for SmallList<T> {
    type Item = T;
    type IntoIter = <Vec<T> as std::iter::IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}
