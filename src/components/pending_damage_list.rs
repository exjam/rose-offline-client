use bevy::prelude::Component;
use rose_game_common::{data::Damage, messages::ClientEntityId};

pub struct PendingDamage {
    pub age: f32,
    pub attacker: ClientEntityId,
    pub damage: Damage,
    pub is_kill: bool,
    pub is_immediate: bool,
}

#[derive(Component)]
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

impl PendingDamageList {
    pub fn add(&mut self, attacker: ClientEntityId, damage: Damage, is_kill: bool, is_immediate: bool) {
        self.pending_damage.push(PendingDamage {
            age: 0.0,
            attacker,
            damage,
            is_kill,
            is_immediate,
        });
    }
}
