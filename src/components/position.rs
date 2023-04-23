use bevy::{
    prelude::{Component, Deref, DerefMut, Vec3},
    reflect::{FromReflect, Reflect},
};

#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect, FromReflect)]
pub struct Position {
    pub position: Vec3,
}

impl Position {
    pub fn new(position: Vec3) -> Self {
        Self { position }
    }
}
