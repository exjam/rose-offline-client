use bevy::prelude::{Component, Entity};
use rose_data::EffectFileId;

#[derive(Component)]
pub struct Projectile {
    pub source: Entity,
    pub hit_effect_file_id: Option<EffectFileId>,
}

impl Projectile {
    pub fn new(source: Entity, hit_effect_file_id: Option<EffectFileId>) -> Self {
        Self {
            source,
            hit_effect_file_id,
        }
    }
}
