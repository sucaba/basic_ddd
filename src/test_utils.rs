#![cfg(test)]

use crate::abstractions::Streamable;

pub(crate) trait AssumeChangesSaved {
    fn assume_changes_saved(&mut self);
}

impl<T: Streamable> AssumeChangesSaved for T {
    fn assume_changes_saved(&mut self) {
        drop(self.commit_changes())
    }
}
