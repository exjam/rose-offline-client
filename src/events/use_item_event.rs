use bevy::prelude::{Entity, Event};

use rose_data::ItemReference;

#[derive(Event)]
pub struct UseItemEvent {
    pub entity: Entity,
    pub item: ItemReference,
}
