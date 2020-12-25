use crate::changable::Changable;
use crate::changes::{FullChange, FullChanges, Record};
use std::mem;

pub trait Undoable: Changable + Sized {
    fn changes_mut(&mut self) -> &mut Record<Self::EventType>;

    fn begin_changes(&mut self) -> Atomic<'_, Self> {
        let check_point = self.changes_mut().history_len();
        Atomic {
            subj: self,
            check_point,
        }
    }

    fn undo_manager<'a>(&'a mut self) -> UndoManager<'a, Self> {
        UndoManager { subj: self }
    }

    fn forget_changes(&mut self) {
        mem::take(self.changes_mut());
    }
}

pub struct Atomic<'a, T: Undoable> {
    subj: &'a mut T,
    check_point: usize,
}

impl<'a, T: Undoable> Atomic<'a, T> {
    pub fn invoke<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        f(self.subj)
    }

    pub fn mutate<F, E>(&mut self, f: F) -> Result<(), E>
    where
        F: FnOnce(&mut T) -> Result<FullChanges<T::EventType>, E>,
    {
        let changes = f(self.subj)?;
        self.subj.changes_mut().extend(changes);
        Ok(())
    }

    pub fn mutate_inner<InnerEvent, M, B, E>(
        &mut self,
        change_inner: M,
        bubble_up: B,
    ) -> Result<(), E>
    where
        M: FnOnce(&mut T) -> Result<FullChanges<InnerEvent>, E>,
        B: Clone + Fn(InnerEvent) -> T::EventType,
    {
        let inner_changes = change_inner(self.subj)?;
        let changes = inner_changes.bubble_up(bubble_up);
        self.subj.changes_mut().extend(changes);

        Ok(())
    }

    pub fn commit(self) {
        mem::forget(self)
    }
}

pub struct UndoManager<'a, T: Undoable> {
    subj: &'a mut T,
}

impl<'a, T: Undoable> UndoManager<'a, T> {
    fn changes_mut(&mut self) -> &mut Record<T::EventType> {
        self.subj.changes_mut()
    }

    pub fn undo(&mut self) -> bool
    where
        T::EventType: Clone,
    {
        if let Some(c) = self.changes_mut().pop_undo() {
            let change = Record::make_applied(c.take_undo(), |e| self.subj.apply(e));
            self.changes_mut().push_redo(change);
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool
    where
        T::EventType: Clone,
    {
        if let Some(c) = self.changes_mut().pop_redo() {
            let change = Record::make_applied(c.take_undo(), |e| self.subj.apply(e));
            self.changes_mut().push_undo(change);
            true
        } else {
            false
        }
    }

    pub fn undo_all(&mut self)
    where
        T::EventType: Clone,
    {
        while self.undo() {}
    }

    pub fn redo_n(&mut self, n: usize)
    where
        T::EventType: Clone,
    {
        for _ in 0..n {
            self.redo();
        }
    }

    pub fn forget_changes(&mut self) {
        mem::take(self.changes_mut());
    }

    pub fn iter_last_redos(
        &mut self,
        count: usize,
    ) -> impl '_ + DoubleEndedIterator<Item = &FullChange<T::EventType>>
    where
        T::EventType: Clone,
    {
        self.changes_mut().iter_last_redos(count)
    }

    pub fn iter_undos(&mut self) -> impl '_ + DoubleEndedIterator<Item = &FullChange<T::EventType>>
    where
        T::EventType: Clone,
    {
        self.changes_mut().iter_undos()
    }
}

impl<'a, T: Undoable> Drop for Atomic<'a, T> {
    fn drop(&mut self) {
        let mut to_compensate: Vec<_> = self
            .subj
            .changes_mut()
            .take_after(self.check_point)
            .collect();
        to_compensate.reverse();
        for c in to_compensate {
            self.subj.apply(c.take_undo());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streamable::Streamable;
    use pretty_assertions::assert_eq;
    use TestEvent::*;

    #[derive(Debug, Eq, PartialEq)]
    struct TestEntry {
        state: TestEvent,
        changes: Record<TestEvent>,
    }

    #[derive(Copy, Clone, Debug, Eq, PartialEq)]
    enum TestEvent {
        Stopped,
        Started,
        Paused,
    }

    impl TestEntry {
        /// Example of atomic operation which fails in the middle
        /// and causes compensation logic to restore state before
        /// the action start
        fn double_start(&mut self) -> crate::result::Result<()> {
            let mut trx = self.begin_changes();

            trx.invoke(Self::start)?;
            trx.invoke(Self::start)?; // fail and rollback both starts

            trx.commit();
            Ok(())
        }

        fn start(&mut self) -> Result<(), String> {
            self.validate_not_started()?;

            let change = Record::make_applied(Started, |e| self.apply(e));
            self.changes.push_undo(change);
            Ok(())
        }

        fn validate_not_started(&self) -> Result<(), String> {
            if let Started = &self.state {
                return Err("Already started".into());
            }
            Ok(())
        }
    }

    impl Changable for TestEntry {
        type EventType = TestEvent;

        fn apply(&mut self, event: Self::EventType) -> Self::EventType {
            match (self.state, event) {
                (Stopped, Started) | (Paused, Started) | (Started, Stopped) | (Started, Paused) => {
                    let undo = self.state;
                    self.state = event;
                    undo
                }
                _ => panic!("not supported {:#?}", (self.state, event)),
            }
        }
    }

    impl Undoable for TestEntry {
        fn changes_mut(&mut self) -> &mut Record<Self::EventType> {
            &mut self.changes
        }
    }

    fn given_stopped() -> TestEntry {
        let sut = TestEntry {
            state: Stopped,
            changes: Record::new(),
        };

        assert_eq!(Stopped, sut.state);
        sut
    }

    #[test]
    fn should_apply_change() {
        let mut sut = given_stopped();

        sut.start().unwrap();

        assert_eq!(sut.state, Started);
    }

    #[test]
    fn should_undo_change() {
        let mut sut = given_stopped();

        sut.start().unwrap();
        let mut ops = sut.undo_manager();
        ops.undo();

        assert_eq!(sut.state, Stopped);
    }

    #[test]
    fn should_redo_change() {
        let mut sut = given_stopped();

        sut.start().unwrap();

        let mut ops = sut.undo_manager();

        ops.undo();

        ops.redo();

        assert_eq!(sut.state, Started);
    }

    #[test]
    fn should_implicitly_rollback_changes() {
        let mut sut = given_stopped();

        assert_eq!(
            sut.double_start(),
            Err("Already started".to_string().into())
        );

        assert_eq!(Stopped, sut.state);

        let changes = sut.take_changes();
        assert_eq!(Vec::<TestEvent>::new(), changes);
    }
}
