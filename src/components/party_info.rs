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

#[derive(Component)]
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
