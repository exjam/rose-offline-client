use bevy::prelude::{Component, Entity};

use rose_data::{EffectBulletMoveType, EffectFileId, SkillId};

#[derive(Component)]
pub struct Projectile {
    pub source: Entity,
    pub skill_id: Option<SkillId>,
    pub move_type: EffectBulletMoveType,
    pub hit_effect_file_id: Option<EffectFileId>,
    pub parabola_velocity: Option<f32>,
}

impl Projectile {
    pub fn new(
        source: Entity,
        skill_id: Option<SkillId>,
        move_type: EffectBulletMoveType,
        hit_effect_file_id: Option<EffectFileId>,
    ) -> Self {
        Self {
            source,
            skill_id,
            move_type,
            hit_effect_file_id,
            parabola_velocity: None,
        }
    }
}
