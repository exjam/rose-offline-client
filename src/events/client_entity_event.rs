use bevy::prelude::Entity;
use rose_data::ItemReference;

#[derive(Copy, Clone, Debug)]
pub enum ClientEntityEvent {
    Die(Entity),
    LevelUp(Entity, u32),
    UseItem(Entity, ItemReference),
}
