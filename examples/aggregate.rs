#![allow(dead_code)]

use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;
use tcache::typeset::SingletonSet;

use basic_ddd::{
    DbOwnedEvent, DbPrimaryEvent, Error, Id, Identifiable, Owned, OwnedCollection, Primary, Result,
    StreamEvents, Streamable,
};

#[derive(Default, Debug, Eq, PartialEq, Clone)]
struct Order
where
    Self: Streamable,
{
    primary: Primary<OrderPrimary>,
    items: OwnedCollection<Rc<OrderItem>>,

    changes: Vec<<Self as Streamable>::EventType>,
}

#[derive(Debug, Eq, PartialEq, Clone)]
enum OrderEvent {
    Primary(<Primary<OrderPrimary> as Streamable>::EventType),
    Item(<OwnedCollection<Rc<OrderItem>> as Streamable>::EventType),
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
struct OrderPrimary {
    id: i32,
    item_count: usize,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct OrderItem {
    id: i32,
}

impl Identifiable for Order {
    type IdType = <OrderPrimary as Identifiable>::IdType;

    fn id(&self) -> Id<Order> {
        self.primary.get().id().convert()
    }
}

impl Streamable for Order {
    type EventType = OrderEvent;

    fn stream_to<S>(&mut self, stream: &mut S)
    where
        S: StreamEvents<Self::EventType>,
    {
        stream.flush(&mut self.primary);
        stream.flush(&mut self.items)
    }
}

impl From<DbPrimaryEvent<OrderPrimary>> for OrderEvent {
    fn from(src: DbPrimaryEvent<OrderPrimary>) -> Self {
        OrderEvent::Primary(src)
    }
}

impl From<DbOwnedEvent<Rc<OrderItem>>> for OrderEvent {
    fn from(src: DbOwnedEvent<Rc<OrderItem>>) -> Self {
        OrderEvent::Item(src)
    }
}

impl Order {
    fn new(mut primary: OrderPrimary) -> Self {
        primary.item_count = 0;

        Self {
            primary: Primary::new(primary.into()),
            items: Default::default(),
            changes: Default::default(),
        }
    }

    fn item_count(&self) -> usize {
        self.primary.get().item_count
    }

    /*
     * Add item by preserving inner invariant:
     * `item_count` should always match `items.len()`
     */
    fn add_new_item(&mut self, item: Rc<OrderItem>) -> Result<()> {
        self.items.add_new(item)?;
        self.primary
            .update(|mut p| {
                p.item_count += 1;
                p
            })
            .unwrap();
        Ok(())
    }
}

impl Identifiable for OrderPrimary {
    type IdType = i32;

    fn id(&self) -> Id<Self> {
        Id::new(self.id)
    }
}

impl Owned for OrderItem {
    type OwnerType = OrderPrimary;
}

impl Identifiable for OrderItem {
    type IdType = i32;

    fn id(&self) -> Id<Self> {
        Id::new(self.id)
    }
}

struct InMemoryStorage {
    per_type: SingletonSet,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            per_type: SingletonSet::new(),
        }
    }

    pub fn load<T: 'static + Streamable + Clone + Identifiable>(&mut self, id: &Id<T>) -> Result<T>
    where
        Id<T>: Hash,
    {
        let map: &mut HashMap<Id<T>, T> = self.per_type.ensure(Default::default);
        map.get(id)
            .cloned()
            .ok_or_else(|| Error::from_text("does not exist".into()))
    }

    pub fn save<T: 'static + Identifiable>(&mut self, root: T) -> Result<()>
    where
        Id<T>: Hash,
    {
        let map: &mut HashMap<Id<T>, T> = self.per_type.ensure(Default::default);
        map.insert(root.id(), root);
        Ok(())
    }
}

fn main() -> Result<()> {
    let order = create_new_order()?;
    println!("created: {:#?}", order);

    let mut storage = InMemoryStorage::new();
    storage.save(order.clone())?;
    println!("saved");

    let copy = storage.load(&order.id())?;

    println!("loaded");
    pretty_assertions::assert_eq!(order, copy);

    Ok(())
}

fn create_new_order() -> Result<Order> {
    let mut aggregate = Order::new(OrderPrimary {
        id: 42,
        item_count: 777, // ignored
    });
    aggregate.add_new_item(OrderItem { id: 1001 }.into())?;

    // Following causes: Already exists ... error
    // aggregate.add_new_item(OrderItem { id: 1001 }.into())?;

    assert_eq!(1, aggregate.item_count());
    println!("events:\n{:#?}", aggregate.commit_changes());
    Ok(aggregate)
}
