use bevy::{
    prelude::Component,
    reflect::{FromReflect, Reflect},
};

#[derive(Component, Reflect, FromReflect)]
pub struct DummyBoneOffset {
    pub index: usize,
}

impl DummyBoneOffset {
    pub fn new(index: usize) -> Self {
        Self { index }
    }
}
