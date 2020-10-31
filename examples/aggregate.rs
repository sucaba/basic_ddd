#![allow(dead_code)]

use std::rc::Rc;

use basic_ddd::{
    Id, Identifiable, InMemoryStorage, Owned, OwnedCollection, Primary, Result, StreamEvents,
    Streamable,
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
    Item(
        Id<OrderPrimary>,
        <OwnedCollection<Rc<OrderItem>> as Streamable>::EventType,
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
            OrderEvent::Item(_id, e) => self.items.apply(e),
        }
    }

    fn stream_to<S>(&mut self, stream: &mut S)
    where
        S: StreamEvents<Self::EventType>,
    {
        let id = self.id().convert();
        stream.flush(&mut self.primary, |primary_event| {
            OrderEvent::Primary(primary_event)
        });
        stream.flush(&mut self.items, |item_event| {
            OrderEvent::Item(id, item_event)
        })
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

fn main() -> Result<()> {
    let mut storage = InMemoryStorage::new();

    let order0 = create_new_order(0)?;
    storage.save(order0)?;
    let order1 = create_new_order(1)?;
    storage.save(order1)?;

    let mut order42 = create_new_order(42)?;
    println!("created: {:#?}", order42);

    storage.save(order42.clone())?;
    println!("saved");

    let copy = storage.load(&order42.id())?;

    println!("loaded");
    let _ = order42.commit_changes();
    pretty_assertions::assert_eq!(order42, copy);

    Ok(())
}

fn create_new_order(id: i32) -> Result<Order> {
    let mut aggregate = Order::new(OrderPrimary {
        id,
        item_count: 777, // ignored
    });
    aggregate.add_new_item(OrderItem { id: 1001 }.into())?;

    // Following causes: Already exists ... error
    // aggregate.add_new_item(OrderItem { id: 1001 }.into())?;

    //assert_eq!(1, aggregate.item_count());
    //println!("events:\n{:#?}", aggregate.commit_changes());
    Ok(aggregate)
}
