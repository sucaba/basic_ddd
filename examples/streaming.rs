#![allow(dead_code)]

use basic_ddd::{
    DbOwnedEvent, DbPrimaryEvent, HasId, HasOwner, Id, OwnedCollection, Primary, StreamEvents,
    Streamable,
};

#[derive(Debug)]
enum OrderEvent {
    Primary(DbPrimaryEvent<OrderPrimary>),
    Item(DbOwnedEvent<OrderItem>),
}

impl From<DbPrimaryEvent<OrderPrimary>> for OrderEvent {
    fn from(src: DbPrimaryEvent<OrderPrimary>) -> Self {
        OrderEvent::Primary(src)
    }
}

impl From<DbOwnedEvent<OrderItem>> for OrderEvent {
    fn from(src: DbOwnedEvent<OrderItem>) -> Self {
        OrderEvent::Item(src)
    }
}

#[derive(Debug, Clone)]
struct OrderPrimary {
    id: i32,
}

#[derive(Debug, Clone)]
struct OrderItem {
    id: i32,
}

impl HasId for OrderPrimary {
    type IdType = i32;

    fn id(&self) -> Id<Self> {
        Id::new(self.id)
    }
}

impl HasOwner for OrderItem {
    type OwnerType = OrderPrimary;
}

impl HasId for OrderItem {
    type IdType = i32;

    fn id(&self) -> Id<Self> {
        Id::new(self.id)
    }
}

#[derive(Default, Debug)]
struct Order {
    primary: Primary<OrderPrimary>,
    items: OwnedCollection<OrderItem>,
    changes: Vec<OrderEvent>,
}

impl Order {
    fn new(primary: OrderPrimary) -> Self {
        Self {
            primary: Primary::new(primary),
            items: Default::default(),
            changes: Default::default(),
        }
    }

    fn add_item(&mut self, item: OrderItem) {
        self.items.add(item)
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

fn main() {
    let mut aggregate = Order::new(OrderPrimary { id: 42 });
    aggregate.add_item(OrderItem { id: 1001 });

    println!("events:\n{:#?}", aggregate.commit_changes());
}
