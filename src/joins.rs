use itertools::{EitherOrBoth, Itertools};

use crate::identifiable::{Identifiable, Id};

pub trait ManyReferences<OtherType: Identifiable> {
    type Iter: Iterator<Item = Id<OtherType>>;

    fn references(&self) -> Self::Iter;
}

pub trait SingleReference<OtherType: Identifiable> {
    fn reference(&self) -> Id<OtherType>;
}

pub fn join<'a, R, D, Refs, Defs>(
    refs: Refs,
    defs: Defs,
) -> impl IntoIterator<Item = (&'a R, &'a D)>
where
    Refs: IntoIterator<Item = &'a R>,
    Defs: IntoIterator<Item = &'a D>,
    R: 'a + SingleReference<D>,
    D: 'a + Identifiable,
    D::IdType: Ord,
{
    refs.into_iter()
        .merge_join_by(defs, |x, y| {
            Ord::cmp(&x.reference(), &y.id())
        })
        .filter_map(|e| match e {
            EitherOrBoth::Both(x, y) => Some((x, y)),
            _ => None,
        })
}
