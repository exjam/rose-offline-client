use bevy::{
    prelude::{Component, Entity},
    reflect::Reflect,
};

#[derive(Clone, Component, Reflect)]
pub struct PersonalStore {
    pub title: String,
    pub skin: usize,
}

impl PersonalStore {
    pub fn new(title: String, skin: usize) -> Self {
        Self { title, skin }
    }
}

#[derive(Clone, Component, Reflect)]
pub struct PersonalStoreModel {
    pub skin: usize,
    pub model: Entity,
    pub model_parts: Vec<Entity>,
}
