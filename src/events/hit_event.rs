use bevy::prelude::Entity;
use rose_data::SkillId;

pub enum HitEventType {
    Weapon,
    Skill(SkillId),
}

pub struct HitEvent {
    pub attacker: Entity,
    pub defender: Entity,
    pub event_type: HitEventType,
    pub apply_damage: bool,
    pub apply_skill_effect: bool,
}

impl HitEvent {
    pub fn with_weapon(attacker: Entity, defender: Entity) -> Self {
        Self {
            attacker,
            defender,
            event_type: HitEventType::Weapon,
            apply_damage: true,
            apply_skill_effect: false,
        }
    }

    pub fn with_skill(attacker: Entity, defender: Entity, skill_id: SkillId) -> Self {
        Self {
            attacker,
            defender,
            event_type: HitEventType::Skill(skill_id),
            apply_damage: true,
            apply_skill_effect: true,
        }
    }
}
