use bevy::prelude::{Component, Deref, DerefMut};

use rose_data::SkillId;
use rose_game_common::{data::Damage, messages::ClientEntityId};

pub struct PendingDamage {
    pub age: f32,
    pub attacker: ClientEntityId,
    pub damage: Damage,
    pub is_kill: bool,
    pub is_immediate: bool,
    pub from_skill: Option<(SkillId, i32)>,
}

impl PendingDamage {
    pub fn new(
        attacker: ClientEntityId,
        damage: Damage,
        is_kill: bool,
        is_immediate: bool,
        from_skill: Option<(SkillId, i32)>,
    ) -> Self {
        Self {
            age: 0.0,
            attacker,
            damage,
            is_kill,
            is_immediate,
            from_skill,
        }
    }
}

#[derive(Component, Deref, DerefMut)]
pub struct PendingDamageList {
    pub pending_damage: Vec<PendingDamage>,
}

impl Default for PendingDamageList {
    fn default() -> Self {
        Self {
            pending_damage: Vec::with_capacity(32),
        }
    }
}
