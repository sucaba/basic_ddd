mod abstractions;
mod owned_collection;
mod primary;
pub mod result;
mod test_utils;

pub use abstractions::*;
pub use owned_collection::*;
pub use primary::*;
pub use result::*;

pub trait ManyReferences<OtherType: Identifiable> {
    type Iter: Iterator<Item = Id<OtherType>>;

    fn references(&self) -> Self::Iter;
}

pub trait SingleReference<OtherType: Identifiable> {
    fn reference(&self) -> Id<OtherType>;
}
