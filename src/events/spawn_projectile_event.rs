use bevy::prelude::Entity;

use rose_data::{EffectBulletMoveType, EffectId, SkillId};

use crate::components::ProjectileTarget;

pub struct SpawnProjectileEvent {
    pub effect_id: EffectId,

    pub source: Entity,
    pub source_dummy_bone_id: Option<usize>,
    pub source_skill_id: Option<SkillId>,
    pub target: ProjectileTarget,

    pub move_type: EffectBulletMoveType,
    pub move_speed: f32,

    pub apply_damage: bool,
}
