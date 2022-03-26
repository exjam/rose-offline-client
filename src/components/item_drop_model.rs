use bevy::prelude::{Component, Entity};
use rose_game_common::components::DroppedItem;

#[derive(Component)]
pub struct ItemDropModel {
    pub dropped_item: Option<DroppedItem>,
    pub root_bone: Entity,
    pub model_parts: Vec<Entity>,
}
