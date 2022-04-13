use bevy::prelude::{Component, Entity};

use rose_data::SkillId;

pub struct PendingSkillEffect {
    pub age: f32,
    pub skill_id: SkillId,
    pub caster_entity: Option<Entity>,
    pub caster_intelligence: i32,
    pub effect_success: [bool; 2],
}

pub struct PendingSkillTarget {
    pub age: f32,
    pub skill_id: SkillId,
    pub defender_entity: Entity,
}

#[derive(Component)]
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

#[derive(Component)]
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

impl PendingSkillEffectList {
    pub fn add_effect(
        &mut self,
        skill_id: SkillId,
        caster_entity: Option<Entity>,
        caster_intelligence: i32,
        effect_success: [bool; 2],
    ) {
        self.pending_skill_effects.push(PendingSkillEffect {
            age: 0.0,
            caster_entity,
            caster_intelligence,
            skill_id,
            effect_success,
        });
    }
}

impl PendingSkillTargetList {
    pub fn add_target(&mut self, skill_id: SkillId, defender_entity: Entity) {
        self.pending_skill_targets.push(PendingSkillTarget {
            age: 0.0,
            skill_id,
            defender_entity,
        });
    }
}
