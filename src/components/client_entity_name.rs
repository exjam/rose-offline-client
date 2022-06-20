use bevy::prelude::{Component, Deref, DerefMut};

#[derive(Component, Deref, DerefMut)]
pub struct ClientEntityName {
    pub name: String,
}

impl ClientEntityName {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}
