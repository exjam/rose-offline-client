use bevy::prelude::Component;
use std::num::NonZeroUsize;

use rose_data::ClanMemberPosition;
use rose_game_common::components::{ClanLevel, ClanMark, ClanPoints, ClanUniqueId, Level, Money};

#[derive(Clone)]
pub struct ClanMember {
    pub name: String,
    pub position: ClanMemberPosition,
    pub contribution: ClanPoints,
    pub level: Level,
    pub job: u16,
    pub channel_id: Option<NonZeroUsize>,
}

#[derive(Component)]
pub struct Clan {
    pub unique_id: ClanUniqueId,
    pub name: String,
    pub description: String,
    pub mark: ClanMark,
    pub money: Money,
    pub points: ClanPoints,
    pub level: ClanLevel,
    pub members: Vec<ClanMember>,
}

impl Clan {
    pub fn find_member_mut(&mut self, name: &str) -> Option<&mut ClanMember> {
        self.members.iter_mut().find(|member| member.name == name)
    }
}
