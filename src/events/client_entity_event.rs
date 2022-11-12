use bevy::prelude::Entity;

#[derive(Copy, Clone, Debug)]
pub enum ClientEntityEvent {
    Die(Entity),
    LevelUp(Entity, Option<u32>),
}
