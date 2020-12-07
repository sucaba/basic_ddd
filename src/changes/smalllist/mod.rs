use std::fmt;
use std::fmt::Debug;
use std::ops;
use std::vec;

pub struct SmallList<T> {
    inner: Vec<T>,
}

impl<T> Debug for SmallList<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(&self.inner).finish()
    }
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

    pub fn reverse(&mut self) {
        self.inner.reverse();
    }

    pub fn drain<R>(&mut self, range: R) -> vec::Drain<'_, T>
    where
        R: ops::RangeBounds<usize>,
    {
        self.inner.drain(range)
    }

    pub fn push(&mut self, item: T) {
        self.inner.push(item)
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

impl<T: Clone> Clone for SmallList<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
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

impl<T: PartialEq> PartialEq for SmallList<T> {
    fn eq(&self, other: &Self) -> bool {
        return PartialEq::eq(&self.inner, &other.inner);
    }
}

impl<T: Eq> Eq for SmallList<T> {}

impl<T> std::iter::IntoIterator for SmallList<T> {
    type Item = T;
    type IntoIter = <Vec<T> as std::iter::IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}
