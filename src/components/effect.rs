use bevy::{
    prelude::Component,
    reflect::{FromReflect, Reflect},
};

#[derive(Component, Reflect, FromReflect)]
pub struct Effect {
    pub manual_despawn: bool,
}

impl Effect {
    pub fn new(manual_despawn: bool) -> Self {
        Self { manual_despawn }
    }
}

#[derive(Component, Default, Reflect, FromReflect)]
pub struct EffectMesh {}

#[derive(Component, Default, Reflect, FromReflect)]
pub struct EffectParticle {}
