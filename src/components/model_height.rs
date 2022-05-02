use bevy::prelude::Component;

#[derive(Component)]
pub struct ModelHeight {
    pub height: f32,
}

impl ModelHeight {
    pub fn new(height: f32) -> Self {
        Self { height }
    }
}
