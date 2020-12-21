mod record;
mod smalllist;

pub use record::*;
use smalllist::SmallList;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct BChange<T> {
    pub redo: T,
    pub undo: T,
}

impl<T> BChange<T> {
    pub fn applied<F>(redo: T, make_undo: F) -> Self
    where
        F: FnOnce(T) -> T,
        T: Clone,
    {
        let undo = make_undo(redo.clone());
        BChange::new(redo, undo)
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

    pub fn bubble_up<O, F>(self, f: F) -> BChange<O>
    where
        F: Fn(T) -> O,
    {
        BChange {
            redo: f(self.redo),
            undo: f(self.undo),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct BChanges<T> {
    inner: SmallList<BChange<T>>,
}

impl<T> BChanges<T> {
    pub fn new() -> Self {
        BChanges {
            inner: SmallList::new(),
        }
    }

    pub fn only(item: BChange<T>) -> Self {
        Self {
            inner: SmallList::once(item),
        }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, BChange<T>> {
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

    pub fn map<F, O>(self, f: F) -> BChanges<O>
    where
        F: Fn(BChange<T>) -> BChange<O>,
    {
        self.into_iter().map(f).collect::<BChanges<O>>()
    }

    pub fn bubble_up<O, F>(self, f: F) -> BChanges<O>
    where
        F: Clone + Fn(T) -> O,
    {
        self.into_iter()
            .map(move |ch| ch.bubble_up(f.clone()))
            .collect::<BChanges<O>>()
    }

    pub fn append<I: IntoIterator<Item = BChange<T>>>(&mut self, iter: I) {
        self.inner.extend(iter)
    }
}

impl<T> From<BChanges<T>> for Record<T> {
    fn from(src: BChanges<T>) -> Self {
        src.into_iter().collect()
    }
}

impl<T, I> std::ops::Index<I> for BChanges<T>
where
    I: std::slice::SliceIndex<[BChange<T>]>,
{
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.inner.index(index)
    }
}

impl<T> Into<Vec<BChange<T>>> for BChanges<T> {
    fn into(self) -> Vec<BChange<T>> {
        self.inner.into_iter().collect()
    }
}

impl<T> std::iter::FromIterator<BChange<T>> for BChanges<T> {
    fn from_iter<I: IntoIterator<Item = BChange<T>>>(iter: I) -> Self {
        Self {
            inner: iter.into_iter().collect(),
        }
    }
}

impl<T> std::iter::IntoIterator for BChanges<T> {
    type Item = BChange<T>;
    type IntoIter = <Vec<BChange<T>> as std::iter::IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}
