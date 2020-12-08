mod record;
mod smalllist;

use crate::abstractions::*;
pub use record::*;
use smalllist::SmallList;
use std::fmt;
use std::fmt::Debug;

pub struct BasicChange<T: Changable> {
    pub redo: T::EventType,
    pub undo: T::EventType,
}

impl<T: Changable> BasicChange<T> {
    pub fn take_redo(self) -> T::EventType {
        self.redo
    }

    pub fn take_undo(self) -> T::EventType {
        self.undo
    }

    pub fn bubble_up<O, F>(self, f: F) -> BasicChange<O>
    where
        O: Changable,
        F: Fn(T::EventType) -> O::EventType,
    {
        BasicChange {
            redo: f(self.redo),
            undo: f(self.undo),
        }
    }
}

impl<T: Changable> Clone for BasicChange<T>
where
    T::EventType: Clone,
{
    fn clone(&self) -> Self {
        BasicChange {
            redo: self.redo.clone(),
            undo: self.undo.clone(),
        }
    }
}

pub trait ChangeUnit<T: Changable> {
    fn from_application(event: T::EventType, subj: &mut T) -> Self;
}

impl<T: Changable> ChangeUnit<T> for BasicChange<T>
where
    T::EventType: Clone,
{
    fn from_application(redo: T::EventType, subj: &mut T) -> Self {
        let undo = subj.apply(redo.clone());
        Self { redo, undo }
    }
}

impl<T> Debug for BasicChange<T>
where
    T: Changable,
    T::EventType: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BasicChange")
            .field("redo", &self.redo)
            .field("undo", &self.undo)
            .finish()
    }
}

impl<T> PartialEq for BasicChange<T>
where
    T: Changable,
    T::EventType: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.redo.eq(&other.redo) && self.undo.eq(&other.undo)
    }
}

pub struct Changes<T: Changable> {
    inner: SmallList<BasicChange<T>>,
}

impl<T: Changable> Changes<T> {
    pub fn new() -> Self {
        Changes {
            inner: SmallList::new(),
        }
    }

    pub fn from_application(redo: T::EventType, subj: &mut T) -> Self
    where
        T::EventType: Clone,
    {
        let undo = subj.apply(redo.clone());
        Self::only(BasicChange { redo, undo })
    }

    fn only(item: BasicChange<T>) -> Self {
        Self {
            inner: SmallList::once(item),
        }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, BasicChange<T>> {
        self.inner.iter()
    }

    pub fn take_after(&mut self, pos: usize) -> Self {
        Self {
            inner: self.inner.take_after(pos),
        }
    }

    pub fn to(self, dest: &mut Self) {
        dest.extend_changes(self)
    }

    pub fn push(&mut self, redo: T::EventType, undo: T::EventType) {
        self.inner.push(BasicChange { redo, undo })
    }

    pub fn map<F, O: Changable>(self, f: F) -> Changes<O>
    where
        F: Fn(BasicChange<T>) -> BasicChange<O>,
    {
        self.into_iter().map(f).collect::<Changes<O>>()
    }

    pub fn bubble_up<F, O: Changable>(self, f: F) -> Changes<O>
    where
        F: Clone + Fn(T::EventType) -> O::EventType,
    {
        self.into_iter()
            .map(move |ch| ch.bubble_up(f.clone()))
            .collect::<Changes<O>>()
    }
}

impl<T, I> std::ops::Index<I> for Changes<T>
where
    T: Changable,
    I: std::slice::SliceIndex<[BasicChange<T>]>,
{
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.inner.index(index)
    }
}

pub trait ExtendChanges<O: Changable> {
    fn extend_changes<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = BasicChange<O>>;
}

/*
impl<O, TEvent> ExtendChanges<O> for O
where
    O: Changable<EventType = TEvent>,
    O: Extend<BasicChange<T>,
{
    fn extend_changes<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = BasicChange<O>>,
    {
        self.extend(iter)
    }
}
*/

impl<T: Changable> Default for Changes<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Changable> Clone for Changes<T>
where
    T::EventType: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: Changable> PartialEq for Changes<T>
where
    T::EventType: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&self.inner, &other.inner)
    }
}

impl<T: Changable> Eq for Changes<T> where T::EventType: Eq {}

impl<T: Changable> Debug for Changes<T>
where
    T::EventType: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Changes")
            .field("items", &self.inner)
            .finish()
    }
}

impl<T: Changable> ExtendChanges<T> for Changes<T> {
    fn extend_changes<I: IntoIterator<Item = BasicChange<T>>>(&mut self, iter: I) {
        self.inner.extend(iter)
    }
}

impl<T: Changable> Into<Vec<BasicChange<T>>> for Changes<T> {
    fn into(self) -> Vec<BasicChange<T>> {
        self.inner.into_iter().collect()
    }
}

impl<T: Changable> std::iter::FromIterator<BasicChange<T>> for Changes<T> {
    fn from_iter<I: IntoIterator<Item = BasicChange<T>>>(iter: I) -> Self {
        Self {
            inner: iter.into_iter().collect(),
        }
    }
}

impl<T: Changable> std::iter::IntoIterator for Changes<T> {
    type Item = BasicChange<T>;
    type IntoIter = <Vec<BasicChange<T>> as std::iter::IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}
