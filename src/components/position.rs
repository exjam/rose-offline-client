use bevy::ecs::prelude::Component;
use bevy::math::Vec3;

#[derive(Component, Clone, Debug)]
pub struct Position {
    pub position: Vec3,
}

impl Position {
    pub fn new(position: Vec3) -> Self {
        Self { position }
    }
}
