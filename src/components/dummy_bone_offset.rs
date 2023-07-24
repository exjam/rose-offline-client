use bevy::{prelude::Component, reflect::Reflect};

#[derive(Component, Reflect)]
pub struct DummyBoneOffset {
    pub index: usize,
}

impl DummyBoneOffset {
    pub fn new(index: usize) -> Self {
        Self { index }
    }
}
