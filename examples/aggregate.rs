#![allow(dead_code)]

use std::error::Error as StdError;
use std::rc::Rc;
use std::result::Result as StdResult;

use basic_ddd::{
    Changable, CloneRedoStreamingStrategy, Details, Error, FullChange, FullChanges, Historic, Id,
    Identifiable, InMemoryStorage, Load, Master, MasterEvent, Owned, Record, Result, Save, Stream,
    Streamable, SupportsDeletion, Undoable,
};

fn main() -> StdResult<(), Box<dyn StdError>> {
    let mut storage = InMemoryStorage::new();

    storage.save(create_new_order(0)?)?;

    let mut order42 = create_new_order(42)?;
    storage.save(order42.clone())?;

    storage.save(create_new_order(1)?)?;

    // println!("storage:\n{:#?}", storage);
    let copy = storage.load(&order42.id())?;

    order42.forget_changes();
    pretty_assertions::assert_eq!(order42, copy);

    println!("success!");
    Ok(())
}

fn create_new_order(id: i32) -> Result<Order> {
    let mut order = Order::new(OrderMaster {
        id,
        item_count: 777, // ignored
    });
    order.add_new_item(OrderItem { id: 1001 })?;
    order.add_new_item(OrderItem { id: 1002 })?;
    let _may_be_added = order.add_new_item(OrderItem { id: 1003 });
    assert_eq!(order.item_count(), 2);

    Ok(order)
}

const MAX_ORDER_ITEMS: usize = 2;

#[derive(Default, Debug, Eq, PartialEq, Clone)]
struct Order {
    master: Master<OrderMaster>,
    items: Details<Rc<OrderItem>>,

    changes: Record<FullChange<OrderEvent>>,
}

#[derive(Debug, Eq, PartialEq, Clone)]
enum OrderEvent {
    Primary(<Master<OrderMaster> as Historic>::EventType),
    Item(
        Id<OrderMaster>,
        <Details<Rc<OrderItem>> as Historic>::EventType,
    ),
}

impl SupportsDeletion for OrderEvent {
    fn is_deletion(&self) -> bool {
        matches!(self, OrderEvent::Primary(MasterEvent::Deleted(_)))
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
struct OrderMaster {
    id: i32,
    item_count: usize,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct OrderItem {
    id: i32,
}

impl Order {
    fn new(mut primary: OrderMaster) -> Self {
        primary.item_count = 0;

        let (primary, changes): (_, FullChanges<_>) = Master::new(primary);
        let top_changes = changes.bubble_up(OrderEvent::Primary);
        Self {
            master: primary,
            items: Default::default(),
            changes: top_changes.into(),
        }
    }

    fn item_count(&self) -> usize {
        self.master.get().item_count
    }

    /*
     * Add item by preserving inner invariant:
     * `item_count` should match `items.len()`
     * Item count is limited by `MAX_ORDER_ITEMS`
     */
    fn add_new_item(&mut self, item: impl Into<Rc<OrderItem>>) -> Result<()> {
        let item = item.into();
        let id = self.id().convert();

        let mut trx = self.begin_changes();

        trx.mutate_inner(
            move |subj| subj.items.add_new(item),
            |e| OrderEvent::Item(id, e),
        )?;

        trx.mutate_inner(
            |subj| -> Result<_> {
                subj.validate_item_limit()?;
                Ok(subj.master.update(|p| p.item_count += 1)?)
            },
            OrderEvent::Primary,
        )?;

        trx.commit();
        Ok(())
    }

    fn validate_item_limit(&self) -> Result<()> {
        if self.master.get().item_count == MAX_ORDER_ITEMS {
            Err(Error::from_text("Too many".into()))
        } else {
            Ok(())
        }
    }
}

impl Identifiable for Order {
    type IdType = <OrderMaster as Identifiable>::IdType;

    fn id(&self) -> Id<Order> {
        self.master.get().id().convert()
    }
}

impl Historic for Order {
    type EventType = OrderEvent;
}

impl Changable for Order {
    fn apply(&mut self, event: Self::EventType) -> Self::EventType {
        match event {
            OrderEvent::Primary(e) => OrderEvent::Primary(self.master.apply(e)),
            OrderEvent::Item(id, e) => OrderEvent::Item(id, self.items.apply(e)),
        }
    }
}

impl Streamable for Order {
    fn stream_to<S>(&mut self, stream: &mut S) -> StdResult<(), Box<dyn StdError>>
    where
        S: Stream<Self::EventType>,
    {
        let mut m = CloneRedoStreamingStrategy::new(self);
        m.stream_to(stream)
    }
}

impl Undoable for Order {
    fn changes_mut(&mut self) -> &mut Record<FullChange<Self::EventType>> {
        &mut self.changes
    }
}

impl Identifiable for OrderMaster {
    type IdType = i32;

    fn id(&self) -> Id<Self> {
        Id::new(self.id)
    }
}

impl Owned for OrderItem {
    type OwnerType = OrderMaster;
}

impl Identifiable for OrderItem {
    type IdType = i32;

    fn id(&self) -> Id<Self> {
        Id::new(self.id)
    }
}
