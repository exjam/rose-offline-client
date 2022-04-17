use bevy::{math::Vec3, prelude::Entity};
use rose_data::{EffectBulletMoveType, EffectFileId};
use rose_game_common::components::MoveSpeed;

#[allow(dead_code)]
pub enum SpawnProjectileTarget {
    Entity(Entity),
    Position(Vec3),
}

pub struct SpawnProjectileEvent {
    pub source: Entity,
    pub source_dummy_bone_id: Option<usize>,
    pub target: SpawnProjectileTarget,

    pub move_type: EffectBulletMoveType,
    pub move_speed: MoveSpeed,

    pub projectile_effect_file_id: Option<EffectFileId>,
    pub hit_effect_file_id: Option<EffectFileId>,
}
