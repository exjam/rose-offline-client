use bevy::prelude::{Component, Deref, DerefMut, Vec3};

#[derive(Component, Clone, Debug, Deref, DerefMut)]
pub struct Position {
    pub position: Vec3,
}

impl Position {
    pub fn new(position: Vec3) -> Self {
        Self { position }
    }
}
