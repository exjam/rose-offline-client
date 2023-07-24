use bevy::{prelude::Component, reflect::Reflect};

#[derive(Component, Reflect)]
pub struct ModelHeight {
    pub height: f32,
}

impl ModelHeight {
    pub fn new(height: f32) -> Self {
        Self { height }
    }
}
