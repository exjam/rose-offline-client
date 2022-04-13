use bevy::prelude::Component;

#[derive(Component)]
pub struct Effect {
    pub manual_despawn: bool,
}

impl Effect {
    pub fn new(manual_despawn: bool) -> Self {
        Self { manual_despawn }
    }
}

#[derive(Component, Default)]
pub struct EffectMesh {}

#[derive(Component, Default)]
pub struct EffectParticle {}
