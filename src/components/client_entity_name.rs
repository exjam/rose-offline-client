use bevy::{
    prelude::{Component, Deref, DerefMut},
    reflect::Reflect,
};

#[derive(Component, Deref, DerefMut, Reflect)]
pub struct ClientEntityName {
    pub name: String,
}

impl ClientEntityName {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}
