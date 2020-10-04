mod abstractions;
mod owned_collection;
mod primary;

pub use abstractions::*;
pub use owned_collection::*;
pub use primary::*;

pub trait ManyReferences<OtherType: HasId> {
    type Iter: Iterator<Item = Id<OtherType>>;

    fn references(&self) -> Self::Iter;
}

pub trait SingleReference<OtherType: HasId> {
    fn reference(&self) -> Id<OtherType>;
}