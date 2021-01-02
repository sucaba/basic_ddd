mod changable;
mod change_abs;
mod changes;
mod contextual;
mod details;
mod historic;
mod identifiable;
mod master;
pub mod result;
mod storage;
mod streamable;
mod streaming;
mod streaming_strategies;
mod test_utils;
mod undoable;

pub use changable::*;
pub use changes::*;
pub use contextual::*;
pub use details::*;
pub use historic::*;
pub use identifiable::*;
pub use master::*;
pub use result::*;
pub use storage::*;
pub use streamable::*;
pub use streaming::*;
pub use streaming_strategies::*;
pub use undoable::*;

pub trait ManyReferences<OtherType: Identifiable> {
    type Iter: Iterator<Item = Id<OtherType>>;

    fn references(&self) -> Self::Iter;
}

pub trait SingleReference<OtherType: Identifiable> {
    fn reference(&self) -> Id<OtherType>;
}
