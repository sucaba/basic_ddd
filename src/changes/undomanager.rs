use super::{BasicChange, Changes, ExtendChanges};
use crate::abstractions::*;
use std::fmt;
use std::fmt::Debug;
use std::ops;

pub struct UndoManager<T: Changable> {
    inner: Vec<BasicChange<T>>,
}

impl<T: Changable> UndoManager<T> {
    pub fn new() -> Self {
        UndoManager { inner: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn reverse(&mut self) {
        self.inner.reverse();
    }

    pub fn iter(&self) -> std::slice::Iter<'_, BasicChange<T>> {
        self.inner.iter()
    }

    pub fn take_after(&mut self, pos: usize) -> impl Iterator<Item = BasicChange<T>> + '_ {
        self.inner.drain(pos..)
    }

    pub fn drain<R>(&mut self, range: R) -> impl Iterator<Item = BasicChange<T>> + '_
    where
        R: ops::RangeBounds<usize>,
    {
        self.inner.drain(range)
    }

    pub fn to(self, dest: &mut Self) {
        dest.extend_changes(self)
    }

    pub fn push(&mut self, redo: T::EventType, undo: T::EventType) {
        self.inner.push(BasicChange { redo, undo })
    }

    pub fn map<F, O: Changable>(self, f: F) -> UndoManager<O>
    where
        F: Fn(BasicChange<T>) -> BasicChange<O>,
    {
        self.into_iter().map(f).collect::<UndoManager<O>>()
    }

    pub fn bubble_up<F, O: Changable>(self, f: F) -> UndoManager<O>
    where
        F: Clone + Fn(T::EventType) -> O::EventType,
    {
        self.into_iter()
            .map(move |ch| ch.bubble_up(f.clone()))
            .collect()
    }
}

impl<T, I> std::ops::Index<I> for UndoManager<T>
where
    T: Changable,
    I: std::slice::SliceIndex<[BasicChange<T>]>,
{
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.inner.index(index)
    }
}

impl<T: Changable> Default for UndoManager<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Changable> Clone for UndoManager<T>
where
    T::EventType: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: Changable> PartialEq for UndoManager<T>
where
    T::EventType: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&self.inner, &other.inner)
    }
}

impl<T: Changable> Eq for UndoManager<T> where T::EventType: Eq {}

impl<T: Changable> Debug for UndoManager<T>
where
    T::EventType: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("UndoManager")
            .field("items", &self.inner)
            .finish()
    }
}

impl<T: Changable> ExtendChanges<T> for UndoManager<T> {
    fn extend_changes<I: IntoIterator<Item = BasicChange<T>>>(&mut self, iter: I) {
        self.inner.extend(iter)
    }
}

impl<T: Changable> Into<Vec<BasicChange<T>>> for UndoManager<T> {
    fn into(self) -> Vec<BasicChange<T>> {
        self.inner.into_iter().collect()
    }
}

impl<T: Changable> std::iter::FromIterator<BasicChange<T>> for UndoManager<T> {
    fn from_iter<I: IntoIterator<Item = BasicChange<T>>>(iter: I) -> Self {
        Self {
            inner: iter.into_iter().collect(),
        }
    }
}

impl<T: Changable> std::iter::IntoIterator for UndoManager<T> {
    type Item = BasicChange<T>;
    type IntoIter = <Vec<BasicChange<T>> as std::iter::IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<T: Changable> From<Changes<T>> for UndoManager<T> {
    fn from(changes: Changes<T>) -> Self {
        Self {
            inner: changes.inner.into_iter().collect(),
        }
    }
}
