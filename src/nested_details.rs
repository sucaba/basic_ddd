use crate::changable::Changable;
use crate::change_abs::AppliedChange;
use crate::historic::Historic;
use crate::identifiable::GetId;
use crate::result::{AlreadyExists, NotFound};
use std::error::Error as StdError;
use crate::changes::FullChanges;

/// Represents collection of *nested* aggregates.
/// Behaves as collection and at the same time all mut methods emit changes related to individual
/// entries.
/// Unlike `Details`, it deals with an arbitrary changes of contained items.
#[derive(Debug)]
pub struct NestedDetails<T> {
    items: Vec<T>,
}

impl<T> NestedDetails<T>
where
    T: GetId + Changable,
{
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Adds new item created by `create` function.
    pub fn add_new<C, F>(&mut self, create: F) -> Result<C, AlreadyExists<(T, C)>>
    where
        F: FnOnce() -> (T, C),
    {
        let (item, changes) = create();
        if self.exists(&item.get_id()) {
            Err(AlreadyExists((item, changes)))
        } else {
            self.items.push(item);
            Ok(changes)
        }
    }

    pub fn update<F>(&mut self, id: &T::Id, f: F) -> Result<FullChanges<NestedEvent<T>>, Box<dyn StdError>>
    where
        F: FnOnce(&mut T) -> Result<FullChanges<T::EventType>, Box<dyn StdError>>,
        T: Clone,
        T::Id: Clone
    {
        if let Some(existing) = self.by_id_mut(id) {
            let inner_changes =  f(existing)?;
            Ok(inner_changes.bubble_up(|e| NestedEvent::Updated(id.clone(), e)))
        } else {
            Err(NotFound(()).into())
        }
    }

    pub fn remove<C>(&mut self, id: &T::Id) -> Result<C, Box<dyn StdError>>
    where
        T: CreateDeletedEvent + Default,
        C: AppliedChange<NestedEvent<T>>,
        T::Id: Clone,
    {
        if let Some(pos) = self.pos_by_id(id) {
            let inner_deleted = self.items[pos].create_deleted_event();
            Ok(self.applied(NestedEvent::Deleted(id.clone(), inner_deleted)))
        } else {
            Err(NotFound(()).into())
        }
    }

    pub fn by_id(&mut self, id: &T::Id) -> Option<&T> {
        self.items.iter().find(|x| &x.get_id() == id)
    }

    fn by_id_mut(&mut self, id: &T::Id) -> Option<&mut T> {
        self.items.iter_mut().find(|x| &x.get_id() == id)
    }

    fn pos_by_id(&self, id: &T::Id) -> Option<usize> {
        self.items.iter().position(|x| &x.get_id() == id)
    }

    fn exists(&mut self, id: &T::Id) -> bool {
        self.items.iter().any(|x| &x.get_id() == id)
    }
}

pub trait CreateDeletedEvent: Historic {
    fn create_deleted_event(&self) -> Self::EventType;
}

#[derive(Clone)]
pub enum NestedEvent<T: Historic + GetId> {
    Noop,
    Created(T::Id, T::EventType),
    Updated(T::Id, T::EventType),
    Deleted(T::Id, T::EventType),
}

impl<T: Historic + GetId> Historic for NestedDetails<T> {
    type EventType = NestedEvent<T>;
}

impl<T: GetId + Default + Changable> Changable for NestedDetails<T> {
    fn apply(&mut self, event: Self::EventType) -> Self::EventType {
        use NestedEvent::*;
        match event {
            Noop => Noop,
            Created(id, e) => {
                if self.exists(&id) {
                    Noop
                } else {
                    let mut item = T::default();
                    let undo = item.apply(e);
                    self.items.push(item);
                    Deleted(id, undo)
                }
            }
            Updated(id, e) => {
                if let Some(item) = self.by_id_mut(&id) {
                    Updated(item.get_id(), item.apply(e))
                } else {
                    Noop
                }
            }
            Deleted(id, e) => {
                if let Some(pos) = self.pos_by_id(&id) {
                    let mut existing = self.items.remove(pos);
                    Created(id, existing.apply(e))
                } else {
                    Noop
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::changable::Changable;
    use crate::identifiable::{Id, Identifiable};
    use crate::{change_abs::AppliedChange, changes::FullChanges};

    type GenericResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

    #[derive(Debug, Clone)]
    enum NestedObjEvent {
        Created(i32, String),
        NameUpdated(i32, String),
        Deleted(i32),
    }

    #[derive(Debug, Eq, PartialEq, Default, Clone)]
    struct NestedObj {
        id: i32,
        name: String,
    }

    impl NestedObj {
        pub fn new<C>(id: i32, name: String) -> (Self, C)
        where
            C: AppliedChange<NestedObjEvent>,
        {
            let mut result = Self {
                id: Default::default(),
                name: Default::default(),
            };
            let changes =
                C::from_application(NestedObjEvent::Created(id, name), |e| result.apply(e));

            (result, changes)
        }

        pub fn set_name<C>(&mut self, name: String) -> GenericResult<C>
        where
            C: AppliedChange<NestedObjEvent>,
        {
            Ok(self.applied(NestedObjEvent::NameUpdated(self.id, name)))
        }

        #[allow(dead_code)]
        pub fn delete<C>(&mut self) -> GenericResult<C>
        where
            C: AppliedChange<NestedObjEvent>,
        {
            Ok(self.applied(NestedObjEvent::Deleted(self.id)))
        }
    }

    impl CreateDeletedEvent for NestedObj {
        fn create_deleted_event(&self) -> Self::EventType {
            NestedObjEvent::Deleted(self.id)
        }
    }

    impl Identifiable for NestedObj {
        type IdType = i32;

        fn id(&self) -> Id<Self> {
            Id::new(self.id)
        }
    }

    impl Historic for NestedObj {
        type EventType = NestedObjEvent;
    }

    impl Changable for NestedObj {
        fn apply(&mut self, event: Self::EventType) -> Self::EventType {
            use NestedObjEvent::*;
            match event {
                Created(id, name) => {
                    self.id = id;
                    self.name = name;
                    Deleted(id)
                }
                NameUpdated(id, name) => {
                    let result = NameUpdated(id, std::mem::take(&mut self.name));
                    self.name = name;
                    result
                }
                Deleted(id) => Created(id, std::mem::take(&mut self.name)),
            }
        }
    }

    #[test]
    fn should_create_new() {
        let mut sut = NestedDetails::new();
        let _changes: FullChanges<NestedObjEvent> =
            sut.add_new(|| NestedObj::new(42, "test1".into())).unwrap();

        assert_eq!(
            sut.by_id(&Id::new(42)),
            Some(&NestedObj::new::<FullChanges<NestedObjEvent>>(42, "test1".into()).0)
        );
    }

    #[test]
    fn should_update_existing() {
        let mut sut = given_existing(42, "test1");

        let _changes: FullChanges<NestedEvent<NestedObj>> = sut
            .update(&Id::new(42), |x| x.set_name("test2".into()))
            .unwrap();

        assert_eq!(
            sut.by_id(&Id::new(42)),
            Some(&NestedObj::new::<FullChanges<NestedObjEvent>>(42, "test2".into()).0)
        );
    }

    #[test]
    fn should_remove_existing() {
        let mut sut = given_existing(42, "test1");

        let _changes: FullChanges<NestedEvent<NestedObj>> =
            sut.remove(&Id::new(42)).unwrap();

        assert_eq!(sut.by_id(&Id::new(42)), None);
    }

    #[test]
    fn should_apply_created() {
        let mut sut = given_existing(42, "test1");

        assert_eq!(
            sut.by_id(&Id::new(42)),
            Some(&NestedObj::new::<FullChanges<NestedObjEvent>>(42, "test1".into()).0)
        );
    }

    #[test]
    fn should_apply_updated() {
        let mut sut = given_existing(42, "test1");

        sut.apply(NestedEvent::Updated(
            Id::new(42),
            NestedObjEvent::NameUpdated(42, "test2".into()),
        ));

        assert_eq!(
            sut.by_id(&Id::new(42)),
            Some(&NestedObj::new::<FullChanges<NestedObjEvent>>(42, "test2".into()).0)
        );
    }

    #[test]
    fn should_apply_deleted() {
        let mut sut = given_existing(42, "test1");

        sut.apply(NestedEvent::Deleted(
            Id::new(42),
            NestedObjEvent::Deleted(42),
        ));

        assert_eq!(sut.by_id(&Id::new(42)), None);
    }

    type Sut = NestedDetails<NestedObj>;

    fn given_existing(id: i32, name: &'static str) -> Sut {
        let mut result = Sut::new();
        result.apply(NestedEvent::Created(
            Id::new(42),
            NestedObjEvent::Created(id, name.into()),
        ));
        result
    }
}
