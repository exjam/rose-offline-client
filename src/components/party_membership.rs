use bevy::prelude::Component;

use rose_game_common::{
    components::CharacterUniqueId,
    messages::{server::PartyMemberInfo, PartyItemSharing, PartyXpSharing},
};

pub enum PartyOwner {
    Unknown,
    Player,
    Character(CharacterUniqueId),
}

pub struct PartyInfo {
    pub owner: PartyOwner,
    pub members: Vec<PartyMemberInfo>,
    pub item_sharing: PartyItemSharing,
    pub xp_sharing: PartyXpSharing,
}

impl Default for PartyInfo {
    fn default() -> Self {
        Self {
            owner: PartyOwner::Unknown,
            members: Vec::new(),
            item_sharing: PartyItemSharing::EqualLootDistribution,
            xp_sharing: PartyXpSharing::EqualShare,
        }
    }
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
