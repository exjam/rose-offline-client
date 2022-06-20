use bevy::prelude::{Component, Deref, DerefMut, Entity};

use rose_data::SkillId;

pub struct PendingSkillEffect {
    pub age: f32,
    pub skill_id: SkillId,
    pub caster_entity: Option<Entity>,
    pub caster_intelligence: i32,
    pub effect_success: [bool; 2],
}

impl PendingSkillEffect {
    pub fn new(
        skill_id: SkillId,
        caster_entity: Option<Entity>,
        caster_intelligence: i32,
        effect_success: [bool; 2],
    ) -> Self {
        Self {
            age: 0.0,
            caster_entity,
            caster_intelligence,
            skill_id,
            effect_success,
        }
    }
}

#[derive(Component, Deref, DerefMut)]
pub struct PendingSkillEffectList {
    // Pending incoming skill effects
    pub pending_skill_effects: Vec<PendingSkillEffect>,
}

impl Default for PendingSkillEffectList {
    fn default() -> Self {
        Self {
            pending_skill_effects: Vec::with_capacity(32),
        }
    }
}

pub struct PendingSkillTarget {
    pub age: f32,
    pub skill_id: SkillId,
    pub defender_entity: Entity,
}

impl PendingSkillTarget {
    pub fn new(skill_id: SkillId, defender_entity: Entity) -> Self {
        Self {
            age: 0.0,
            skill_id,
            defender_entity,
        }
    }
}

#[derive(Component, Deref, DerefMut)]
pub struct PendingSkillTargetList {
    // Pending outgoing skill effects
    pub pending_skill_targets: Vec<PendingSkillTarget>,
}

impl Default for PendingSkillTargetList {
    fn default() -> Self {
        Self {
            pending_skill_targets: Vec::with_capacity(32),
        }
    }
}
