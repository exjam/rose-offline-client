use bevy::prelude::Component;

#[derive(Component)]
pub struct DummyBoneOffset {
    pub index: usize,
}

impl DummyBoneOffset {
    pub fn new(index: usize) -> Self {
        Self { index }
    }
}
