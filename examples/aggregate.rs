#![allow(dead_code)]

use std::rc::Rc;

use basic_ddd::{
    Changable, Error, FullChange, Id, Identifiable, InMemoryStorage, Load, Owned, OwnedCollection,
    Primary, Record, Result, Save, Undoable,
};

fn main() -> Result<()> {
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
    let mut order = Order::new(OrderPrimary {
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
    primary: Primary<OrderPrimary>,
    items: OwnedCollection<Rc<OrderItem>>,

    changes: Record<FullChange<OrderEvent>>,
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

        let (primary, changes) = Primary::new(primary);
        let top_changes = changes.bubble_up(OrderEvent::Primary);
        Self {
            primary,
            items: Default::default(),
            changes: top_changes.into(),
        }
    }

    fn item_count(&self) -> usize {
        self.primary.get().item_count
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
                Ok(subj.primary.update(|p| p.item_count += 1)?)
            },
            OrderEvent::Primary,
        )?;

        trx.commit();
        Ok(())
    }

    fn validate_item_limit(&self) -> Result<()> {
        if self.primary.get().item_count == MAX_ORDER_ITEMS {
            Err(Error::from_text("Too many".into()))
        } else {
            Ok(())
        }
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

    fn apply(&mut self, event: Self::EventType) -> Self::EventType {
        match event {
            OrderEvent::Primary(e) => OrderEvent::Primary(self.primary.apply(e)),
            OrderEvent::Item(id, e) => OrderEvent::Item(id, self.items.apply(e)),
        }
    }
}

impl Undoable for Order {
    fn changes_mut(&mut self) -> &mut Record<FullChange<Self::EventType>> {
        &mut self.changes
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
