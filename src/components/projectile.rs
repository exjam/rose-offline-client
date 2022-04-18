use bevy::{
    math::Vec3,
    prelude::{Component, Entity},
};

use rose_data::{EffectBulletMoveType, EffectId, SkillId};

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
    pub source: Entity,
    pub effect_id: Option<EffectId>,
    pub skill_id: Option<SkillId>,
    pub move_type: EffectBulletMoveType,
    pub apply_damage: bool,
    pub parabola: Option<ProjectileParabola>,
}

impl Projectile {
    pub fn new(
        source: Entity,
        effect_id: Option<EffectId>,
        skill_id: Option<SkillId>,
        move_type: EffectBulletMoveType,
        apply_damage: bool,
    ) -> Self {
        Self {
            source,
            effect_id,
            skill_id,
            move_type,
            apply_damage,
            parabola: None,
        }
    }
}
