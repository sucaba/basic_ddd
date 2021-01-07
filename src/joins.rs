use itertools::{EitherOrBoth, Itertools};

use crate::identifiable::{GetId, Id, Identifiable};

pub trait ManyReferences<OtherType: Identifiable> {
    type Iter: Iterator<Item = Id<OtherType>>;

    fn references(&self) -> Self::Iter;
}

pub trait SingleReference<OtherType: Identifiable> {
    fn reference(&self) -> Id<OtherType>;
}

pub fn join<'a, R, D, T, Refs, Defs>(
    refs: Refs,
    defs: Defs,
) -> impl IntoIterator<Item = (&'a R, &'a D)>
where
    Refs: IntoIterator<Item = &'a R>,
    Defs: IntoIterator<Item = &'a D>,
    R: 'a + SingleReference<T>,
    D: 'a + GetId<IdentifiableType=T>,
    T: Identifiable,
    Id<T>: Ord,
{
    refs.into_iter()
        .merge_join_by(defs, |x, y| {
            Ord::cmp(&x.reference(), &y.get_id())
        })
        .filter_map(|e| match e {
            EitherOrBoth::Both(x, y) => Some((x, y)),
            _ => None,
        })
}
