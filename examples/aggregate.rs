#![allow(dead_code)]

use std::mem;
use std::rc::Rc;

use basic_ddd::{
    Changable, Changes, DbOwnedEvent, DbPrimaryEvent, Id, Identifiable, InMemoryStorage, Owned,
    OwnedCollection, Primary, Result, StreamEvents, Streamable,
};

type OrderItems = OwnedCollection<Rc<OrderItem>>;
type OrderPrimaryEvent = DbPrimaryEvent<OrderPrimary>;
type OrderItemEvent = DbOwnedEvent<Rc<OrderItem>>;

#[derive(Default, Debug, Eq, PartialEq, Clone)]
struct Order
where
    Self: Streamable,
{
    primary: Primary<OrderPrimary>,
    items: OwnedCollection<Rc<OrderItem>>,

    changes: Changes<Order>,
    completed: bool,
}

#[derive(Debug, Eq, PartialEq, Clone)]
enum OrderEvent {
    Primary(<Primary<OrderPrimary> as Changable>::EventType),
    Item(
        Id<OrderPrimary>,
        <OwnedCollection<Rc<OrderItem>> as Changable>::EventType,
    ),
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

impl Order {
    fn new(mut primary: OrderPrimary) -> Self {
        primary.item_count = 0;

        Self {
            primary: Primary::new(primary.into()),
            items: Default::default(),
            changes: Default::default(),
            completed: true,
        }
    }

    fn item_count(&self) -> usize {
        self.primary.get().item_count
    }

    /*
     * Add item by preserving inner invariant:
     * `item_count` should match `items.len()`
     */
    fn add_new_item(&mut self, item: Rc<OrderItem>) -> Result<()> {
        self.items
            .add_new(item)?
            .ascend_to(self.item_ascension(), &mut self.changes);

        self.primary
            .update(|p| p.item_count += 1)?
            .ascend_to(self.primary_ascension(), &mut self.changes);

        Ok(())
    }

    fn item_ascension(&self) -> impl Fn(OrderItemEvent) -> OrderEvent {
        let id = self.id().convert();
        move |event| OrderEvent::Item(id, event)
    }

    fn primary_ascension(&self) -> impl Fn(OrderPrimaryEvent) -> OrderEvent {
        OrderEvent::Primary
    }
}

impl Identifiable for Order {
    type IdType = <OrderPrimary as Identifiable>::IdType;

    fn id(&self) -> Id<Order> {
        self.primary.get().id().convert()
    }
}

impl Changable for Order {
    type EventType = OrderEvent;

    fn apply(&mut self, event: Self::EventType)
    where
        Self::EventType: Clone,
    {
        if self.completed {
            self.changes.push(event.clone());
        }

        match event {
            OrderEvent::Primary(e) => self.primary.apply(e),
            OrderEvent::Item(_id, e) => self.items.apply(e),
        }
    }
}

impl Streamable for Order {
    fn new_incomplete() -> Self {
        Order {
            primary: Primary::default(),
            items: OwnedCollection::default(),
            changes: Changes::new(),
            completed: false,
        }
    }

    fn mark_complete(&mut self) {
        self.completed = true;
    }

    fn stream_to<S>(&mut self, stream: &mut S)
    where
        S: StreamEvents<Self::EventType>,
    {
        let changes = mem::take(&mut self.changes);
        stream.stream(changes);
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

fn main() -> Result<()> {
    let mut storage = InMemoryStorage::new();

    let order0 = create_new_order(0)?;
    storage.save(order0)?;
    let order1 = create_new_order(1)?;
    storage.save(order1)?;

    let mut order42 = create_new_order(42)?;

    storage.save(order42.clone())?;

    let copy = storage.load(&order42.id())?;

    let _ = order42.commit_changes();
    pretty_assertions::assert_eq!(order42, copy);

    println!("success!");
    Ok(())
}

fn create_new_order(id: i32) -> Result<Order> {
    let mut aggregate = Order::new(OrderPrimary {
        id,
        item_count: 777, // ignored
    });
    aggregate.add_new_item(OrderItem { id: 1001 }.into())?;

    Ok(aggregate)
}
