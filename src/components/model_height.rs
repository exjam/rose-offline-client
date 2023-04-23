use bevy::{
    prelude::Component,
    reflect::{FromReflect, Reflect},
};

#[derive(Component, Reflect, FromReflect)]
pub struct ModelHeight {
    pub height: f32,
}

impl ModelHeight {
    pub fn new(height: f32) -> Self {
        Self { height }
    }
}
