use bevy::prelude::Component;

use rose_game_common::{components::CharacterUniqueId, messages::server::PartyMemberInfo};

pub enum PartyOwner {
    Unknown,
    Player,
    Character(CharacterUniqueId),
}

pub struct PartyInfo {
    pub owner: PartyOwner,
    pub members: Vec<PartyMemberInfo>,
}

#[derive(Component)]
pub enum PartyMembership {
    None,
    Member(PartyInfo),
}

impl PartyMembership {
    pub fn is_none(&self) -> bool {
        matches!(self, PartyMembership::None)
    }
}

impl Default for PartyMembership {
    fn default() -> Self {
        Self::None
    }
}
