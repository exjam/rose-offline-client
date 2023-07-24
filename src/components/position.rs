use bevy::{
    prelude::{Component, Deref, DerefMut, Vec3},
    reflect::Reflect,
};

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
pub struct Position {
    pub position: Vec3,
}

impl Position {
    pub fn new(position: Vec3) -> Self {
        Self { position }
    }
}
