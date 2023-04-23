use bevy::{
    prelude::{Component, Deref, DerefMut},
    reflect::{FromReflect, Reflect},
};

#[derive(Component, Deref, DerefMut, Reflect, FromReflect)]
pub struct ClientEntityName {
    pub name: String,
}

impl ClientEntityName {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}
