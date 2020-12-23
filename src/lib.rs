mod changable;
mod changes;
mod identifiable;
mod owned_collection;
mod primary;
pub mod result;
mod storage;
mod streamable;
mod test_utils;
mod undoable;

pub use changable::*;
pub use changes::*;
pub use identifiable::*;
pub use owned_collection::*;
pub use primary::*;
pub use result::*;
pub use storage::*;
pub use streamable::*;
pub use undoable::*;

pub trait ManyReferences<OtherType: Identifiable> {
    type Iter: Iterator<Item = Id<OtherType>>;

    fn references(&self) -> Self::Iter;
}

pub trait SingleReference<OtherType: Identifiable> {
    fn reference(&self) -> Id<OtherType>;
}
