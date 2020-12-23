#![cfg(test)]

use crate::streamable::Streamable;

pub(crate) trait AssumeChangesSaved {
    fn assume_changes_saved(&mut self);
}

impl<T: Streamable> AssumeChangesSaved for T {
    fn assume_changes_saved(&mut self) {
        drop(self.take_changes())
    }
}
