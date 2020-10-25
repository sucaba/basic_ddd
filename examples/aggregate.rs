#![allow(dead_code)]

use std::hash::Hash;
use std::marker::PhantomData;
use std::rc::Rc;

use basic_ddd::{
    DbOwnedEvent, DbPrimaryEvent, Id, Identifiable, Owned, OwnedCollection, Primary, Result,
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

    fn new_incomplete() -> Self {
        Order {
            primary: Primary::new_incomplete(),
            items: OwnedCollection::new_incomplete(),
            changes: Vec::new(),
        }
    }

    fn apply(&mut self, event: Self::EventType) {
        match event {
            OrderEvent::Primary(e) => self.primary.apply(e),
            OrderEvent::Item(e) => self.items.apply(e),
        }
    }

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

struct InMemoryStorage<T, TEvent>
where
    T: Streamable<EventType = TEvent> + Identifiable,
{
    events: Vec<TEvent>,
    marker: PhantomData<T>,
}

impl<T, TEvent> InMemoryStorage<T, TEvent>
where
    T: Streamable<EventType = TEvent> + Identifiable,
    TEvent: std::fmt::Debug,
{
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            marker: PhantomData,
        }
    }

    pub fn load(&mut self, _id: &Id<T>) -> Result<T>
    where
        Id<T>: Hash,
        TEvent: Clone,
    {
        let mut result = T::new_incomplete();
        for e in &self.events {
            println!("applying {:#?}", e);
            result.apply(e.clone());
        }
        Ok(result)
    }

    pub fn save(&mut self, mut root: T) -> Result<()>
    where
        Id<T>: Hash,
    {
        root.stream_to(self);
        println!("events after save={:#?}", self.events);
        Ok(())
    }
}

impl<T, TEvent> StreamEvents<TEvent> for InMemoryStorage<T, TEvent>
where
    T: Streamable<EventType = TEvent> + Identifiable,
{
    fn stream<I>(&mut self, events: I)
    where
        I: IntoIterator<Item = TEvent>,
    {
        self.events.stream(events);
    }
}

fn main() -> Result<()> {
    let mut order = create_new_order()?;
    println!("created: {:#?}", order);

    let mut storage = InMemoryStorage::new();
    storage.save(order.clone())?;
    let _ = order.commit_changes();
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

    //assert_eq!(1, aggregate.item_count());
    //println!("events:\n{:#?}", aggregate.commit_changes());
    Ok(aggregate)
}
