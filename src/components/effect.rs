use bevy::{prelude::Component, reflect::Reflect};

#[derive(Component, Reflect)]
pub struct Effect {
    pub manual_despawn: bool,
}

impl Effect {
    pub fn new(manual_despawn: bool) -> Self {
        Self { manual_despawn }
    }
}

#[derive(Component, Default, Reflect)]
pub struct EffectMesh {}

#[derive(Component, Default, Reflect)]
pub struct EffectParticle {}
