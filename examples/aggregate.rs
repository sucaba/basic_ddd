#![allow(dead_code)]

use basic_ddd::{
    DbOwnedEvent, DbPrimaryEvent, HasId, HasOwner, Id, OwnedCollection, Primary, StreamEvents,
    Streamable,
};

#[derive(Default, Debug)]
struct Order
where
    Self: Streamable,
{
    primary: Primary<OrderPrimary>,
    items: OwnedCollection<OrderItem>,

    changes: Vec<<Self as Streamable>::EventType>,
}

#[derive(Debug)]
enum OrderEvent {
    Primary(<Primary<OrderPrimary> as Streamable>::EventType),
    Item(<OwnedCollection<OrderItem> as Streamable>::EventType),
}

#[derive(Debug, Clone, Default)]
struct OrderPrimary {
    id: i32,
    item_count: usize,
}

#[derive(Debug, Clone)]
struct OrderItem {
    id: i32,
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

impl From<DbOwnedEvent<OrderItem>> for OrderEvent {
    fn from(src: DbOwnedEvent<OrderItem>) -> Self {
        OrderEvent::Item(src)
    }
}

impl Order {
    fn new(mut primary: OrderPrimary) -> Self {
        primary.item_count = 0;

        Self {
            primary: Primary::new(primary),
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
    fn add_new_item(&mut self, item: OrderItem) -> std::result::Result<(), OrderItem> {
        let result = self.items.add_new(item);
        if result.is_ok() {
            self.primary.update(|mut p| {
                p.item_count += 1;
                p
            });
        }

        result
    }
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

fn main() {
    let mut aggregate = Order::new(OrderPrimary {
        id: 42,
        item_count: 777, // ignored
    });
    aggregate
        .add_new_item(OrderItem { id: 1001 })
        .expect("item already exists");

    assert_eq!(1, aggregate.item_count());
    println!("events:\n{:#?}", aggregate.commit_changes());
}
