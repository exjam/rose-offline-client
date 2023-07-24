use bevy::prelude::{Entity, Event};

use rose_data::{EffectId, SkillId};

#[derive(Event)]
pub struct HitEvent {
    pub attacker: Entity,
    pub defender: Entity,
    pub effect_id: Option<EffectId>,
    pub skill_id: Option<SkillId>,
    pub apply_damage: bool,
    pub ignore_miss: bool,
}

impl HitEvent {
    pub fn with_weapon(attacker: Entity, defender: Entity, effect_id: Option<EffectId>) -> Self {
        Self {
            attacker,
            defender,
            effect_id,
            skill_id: None,
            apply_damage: true,
            ignore_miss: false,
        }
    }

    pub fn with_skill_damage(attacker: Entity, defender: Entity, skill_id: SkillId) -> Self {
        Self {
            attacker,
            defender,
            effect_id: None,
            skill_id: Some(skill_id),
            apply_damage: true,
            ignore_miss: false,
        }
    }

    pub fn with_skill_effect(attacker: Entity, defender: Entity, skill_id: SkillId) -> Self {
        Self {
            attacker,
            defender,
            effect_id: None,
            skill_id: Some(skill_id),
            apply_damage: true,
            ignore_miss: true,
        }
    }

    pub fn apply_damage(mut self, apply_damage: bool) -> Self {
        self.apply_damage = apply_damage;
        self
    }
}
