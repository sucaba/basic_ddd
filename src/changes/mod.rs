mod record;
mod smalllist;

pub use record::*;
use smalllist::SmallList;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Change<T> {
    pub redo: T,
    pub undo: T,
}

impl<T> Change<T> {
    pub fn applied<F>(redo: T, make_undo: F) -> Self
    where
        F: FnOnce(T) -> T,
        T: Clone,
    {
        let undo = make_undo(redo.clone());
        Change::new(redo, undo)
    }

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

    pub fn bubble_up<O, F>(self, f: F) -> Change<O>
    where
        F: Fn(T) -> O,
    {
        Change {
            redo: f(self.redo),
            undo: f(self.undo),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Changes<T> {
    inner: SmallList<Change<T>>,
}

impl<T> Changes<T> {
    pub fn new() -> Self {
        Changes {
            inner: SmallList::new(),
        }
    }

    pub fn only(item: Change<T>) -> Self {
        Self {
            inner: SmallList::once(item),
        }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Change<T>> {
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

    pub fn map<F, O>(self, f: F) -> Changes<O>
    where
        F: Fn(Change<T>) -> Change<O>,
    {
        self.into_iter().map(f).collect::<Changes<O>>()
    }

    pub fn bubble_up<O, F>(self, f: F) -> Changes<O>
    where
        F: Clone + Fn(T) -> O,
    {
        self.into_iter()
            .map(move |ch| ch.bubble_up(f.clone()))
            .collect::<Changes<O>>()
    }

    pub fn append<I: IntoIterator<Item = Change<T>>>(&mut self, iter: I) {
        self.inner.extend(iter)
    }
}

impl<T> From<Changes<T>> for Record<T> {
    fn from(src: Changes<T>) -> Self {
        src.into_iter().collect()
    }
}

impl<T, I> std::ops::Index<I> for Changes<T>
where
    I: std::slice::SliceIndex<[Change<T>]>,
{
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.inner.index(index)
    }
}

impl<T> Into<Vec<Change<T>>> for Changes<T> {
    fn into(self) -> Vec<Change<T>> {
        self.inner.into_iter().collect()
    }
}

impl<T> std::iter::FromIterator<Change<T>> for Changes<T> {
    fn from_iter<I: IntoIterator<Item = Change<T>>>(iter: I) -> Self {
        Self {
            inner: iter.into_iter().collect(),
        }
    }
}

impl<T> std::iter::IntoIterator for Changes<T> {
    type Item = Change<T>;
    type IntoIter = <Vec<Change<T>> as std::iter::IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}
