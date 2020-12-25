mod record;
mod smalllist;

pub use record::*;
use smalllist::SmallList;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FullChange<T> {
    redo: T,
    undo: T,
}

impl<T> FullChange<T> {
    pub fn new(redo: T, undo: T) -> Self {
        Self { undo, redo }
    }

    pub fn take_redo(self) -> T {
        self.redo
    }

    pub fn take_undo(self) -> T {
        self.undo
    }

    pub fn redo(&self) -> &T {
        &self.redo
    }

    pub fn undo(&self) -> &T {
        &self.undo
    }

    pub fn bubble_up<O, F>(self, f: F) -> FullChange<O>
    where
        F: Fn(T) -> O,
    {
        FullChange {
            redo: f(self.redo),
            undo: f(self.undo),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FullChanges<T> {
    inner: SmallList<FullChange<T>>,
}

impl<T> FullChanges<T> {
    pub fn new() -> Self {
        FullChanges {
            inner: SmallList::new(),
        }
    }

    pub fn only(item: FullChange<T>) -> Self {
        Self {
            inner: SmallList::only(item),
        }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, FullChange<T>> {
        self.inner.iter()
    }

    pub fn take_after(&mut self, pos: usize) -> Self {
        Self {
            inner: self.inner.take_after(pos),
        }
    }

    pub fn to(self, dest: &mut Self) {
        dest.append(self)
    }

    pub fn map<F, O>(self, f: F) -> FullChanges<O>
    where
        F: Fn(FullChange<T>) -> FullChange<O>,
    {
        self.into_iter().map(f).collect::<FullChanges<O>>()
    }

    pub fn bubble_up<O, F>(self, f: F) -> FullChanges<O>
    where
        F: Clone + Fn(T) -> O,
    {
        self.into_iter()
            .map(move |ch| ch.bubble_up(f.clone()))
            .collect::<FullChanges<O>>()
    }

    pub fn append<I: IntoIterator<Item = FullChange<T>>>(&mut self, iter: I) {
        self.inner.extend(iter)
    }
}

impl<T> From<FullChanges<T>> for Record<T> {
    fn from(src: FullChanges<T>) -> Self {
        src.into_iter().collect()
    }
}

impl<T, I> std::ops::Index<I> for FullChanges<T>
where
    I: std::slice::SliceIndex<[FullChange<T>]>,
{
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.inner.index(index)
    }
}

impl<T> Into<Vec<FullChange<T>>> for FullChanges<T> {
    fn into(self) -> Vec<FullChange<T>> {
        self.inner.into_iter().collect()
    }
}

impl<T> std::iter::FromIterator<FullChange<T>> for FullChanges<T> {
    fn from_iter<I: IntoIterator<Item = FullChange<T>>>(iter: I) -> Self {
        Self {
            inner: iter.into_iter().collect(),
        }
    }
}

impl<T> std::iter::IntoIterator for FullChanges<T> {
    type Item = FullChange<T>;
    type IntoIter = <Vec<FullChange<T>> as std::iter::IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}
