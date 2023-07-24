use bevy::prelude::{Entity, Event};

#[derive(Event, Copy, Clone, Debug)]
pub enum ClientEntityEvent {
    Die(Entity),
    LevelUp(Entity, Option<u32>),
}
