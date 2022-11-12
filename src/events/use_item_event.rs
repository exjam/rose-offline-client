use bevy::prelude::Entity;

use rose_data::ItemReference;

pub struct UseItemEvent {
    pub entity: Entity,
    pub item: ItemReference,
}
