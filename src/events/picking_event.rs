use bevy::{math::Vec3, prelude::Entity};

pub struct PickingEvent {
    pub entity: Entity,
    pub position: Vec3,
}
