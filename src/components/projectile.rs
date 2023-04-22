use bevy::{
    math::Vec3,
    prelude::{Component, Entity},
};

use rose_data::{EffectBulletMoveType, EffectId, SkillId};

#[derive(Copy, Clone)]
pub enum ProjectileTarget {
    Entity { entity: Entity },
    Position { position: Vec3 },
}

pub struct ProjectileParabola {
    pub start_y: f32,
    pub end_y: f32,
    pub velocity_y: f32,
    pub move_vec: Vec3,
    pub current_time: f32,
    pub total_time: f32,
}

#[derive(Component)]
pub struct Projectile {
    pub target: ProjectileTarget,
    pub source: Entity,

    pub effect_id: Option<EffectId>,
    pub skill_id: Option<SkillId>,

    pub move_type: EffectBulletMoveType,
    pub move_speed: f32,
    pub parabola: Option<ProjectileParabola>,

    pub apply_damage: bool,
}
