use itertools::{EitherOrBoth, Itertools};

use crate::identifiable::GetId;

pub trait ManyReferences<OtherType: GetId> {
    type Iter: Iterator<Item = OtherType::Id>;

    fn references(&self) -> Self::Iter;
}

pub trait SingleReference<OtherType: GetId> {
    fn reference(&self) -> OtherType::Id;
}

pub fn join<'a, R, O, D, Id, Refs, Defs>(
    refs: Refs,
    defs: Defs,
) -> impl IntoIterator<Item = (&'a R, &'a D)>
where
    Refs: IntoIterator<Item = &'a R>,
    Defs: IntoIterator<Item = &'a D>,
    R: 'a + SingleReference<O>,
    D: 'a + GetId<Id = Id>,
    O: 'a + GetId<Id = Id>,
    Id: Ord,
{
    refs.into_iter()
        .merge_join_by(defs, |x, y| Ord::cmp(&x.reference(), &y.get_id()))
        .filter_map(|e| match e {
            EitherOrBoth::Both(x, y) => Some((x, y)),
            _ => None,
        })
}
