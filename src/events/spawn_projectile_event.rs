use bevy::{math::Vec3, prelude::Entity};

use rose_data::{EffectBulletMoveType, EffectId, SkillId};
use rose_game_common::components::MoveSpeed;

#[allow(dead_code)]
pub enum SpawnProjectileTarget {
    Entity(Entity),
    Position(Vec3),
}

pub struct SpawnProjectileEvent {
    pub effect_id: EffectId,

    pub source: Entity,
    pub source_dummy_bone_id: Option<usize>,
    pub source_skill_id: Option<SkillId>,
    pub target: SpawnProjectileTarget,

    pub move_type: EffectBulletMoveType,
    pub move_speed: MoveSpeed,
}
