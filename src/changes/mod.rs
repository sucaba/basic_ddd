mod record;
mod smalllist;

use crate::abstractions::*;
pub use record::*;
use smalllist::SmallList;
use std::fmt;
use std::fmt::Debug;

pub struct BChange<T> {
    pub redo: T,
    pub undo: T,
}

impl<T> BChange<T> {
    pub fn take_redo(self) -> T {
        self.redo
    }

    pub fn take_undo(self) -> T {
        self.undo
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

impl<T> Clone for BChange<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        BChange {
            redo: self.redo.clone(),
            undo: self.undo.clone(),
        }
    }
}

impl<T> Debug for BChange<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BasicChange")
            .field("redo", &self.redo)
            .field("undo", &self.undo)
            .finish()
    }
}

impl<T> PartialEq for BChange<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.redo.eq(&other.redo) && self.undo.eq(&other.undo)
    }
}

pub struct BChanges<T> {
    inner: SmallList<BChange<T>>,
}

impl<T> BChanges<T> {
    pub fn new() -> Self {
        BChanges {
            inner: SmallList::new(),
        }
    }

    pub fn from_application<S>(redo: T, subj: &mut S) -> Self
    where
        S: Changable<EventType = T>,
        T: Clone,
    {
        let undo = subj.apply(redo.clone());
        Self::only(BChange::<T> { redo, undo })
    }

    fn only(item: BChange<T>) -> Self {
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

    pub fn push(&mut self, redo: T, undo: T) {
        self.inner.push(BChange { redo, undo })
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

impl<T, I> std::ops::Index<I> for BChanges<T>
where
    I: std::slice::SliceIndex<[BChange<T>]>,
{
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.inner.index(index)
    }
}

impl<T> Default for BChanges<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for BChanges<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> PartialEq for BChanges<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&self.inner, &other.inner)
    }
}

impl<T> Eq for BChanges<T> where T: Eq {}

impl<T> Debug for BChanges<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BChanges")
            .field("items", &self.inner)
            .finish()
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
