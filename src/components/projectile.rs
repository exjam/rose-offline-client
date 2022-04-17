use bevy::prelude::{Component, Entity};

use rose_data::{EffectBulletMoveType, EffectFileId};

#[derive(Component)]
pub struct Projectile {
    pub source: Entity,
    pub hit_effect_file_id: Option<EffectFileId>,
    pub move_type: EffectBulletMoveType,
}

impl Projectile {
    pub fn new(
        source: Entity,
        move_type: EffectBulletMoveType,
        hit_effect_file_id: Option<EffectFileId>,
    ) -> Self {
        Self {
            move_type,
            source,
            hit_effect_file_id,
        }
    }
}
