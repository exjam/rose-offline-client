use bevy::prelude::Component;

use rose_data::ClanMemberPosition;
use rose_game_common::components::{ClanLevel, ClanMark, ClanPoints, ClanUniqueId};

#[derive(Component)]
pub struct ClanMembership {
    pub clan_unique_id: ClanUniqueId,
    pub mark: ClanMark,
    pub level: ClanLevel,
    pub name: String,
    pub position: ClanMemberPosition,
    pub contribution: ClanPoints,
}
