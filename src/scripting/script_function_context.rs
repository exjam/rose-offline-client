use bevy::{
    ecs::{query::WorldQuery, system::SystemParam},
    prelude::{EventWriter, Query, With},
};

use rose_game_common::components::{
    AbilityValues, BasicStats, CharacterInfo, Equipment, ExperiencePoints, HealthPoints, Inventory,
    Level, ManaPoints, MoveSpeed, QuestState, SkillPoints, Stamina, StatPoints, Team,
    UnionMembership,
};

use crate::{
    components::{ClientEntity, PlayerCharacter},
    events::ChatboxEvent,
};

#[derive(WorldQuery)]
pub struct ScriptCharacterQuery<'w> {
    pub ability_values: &'w AbilityValues,
    pub character_info: &'w CharacterInfo,
    pub basic_stats: &'w BasicStats,
    pub experience_points: &'w ExperiencePoints,
    pub health_points: &'w HealthPoints,
    pub inventory: &'w Inventory,
    pub level: &'w Level,
    pub mana_points: &'w ManaPoints,
    pub move_speed: &'w MoveSpeed,
    pub skill_points: &'w SkillPoints,
    pub stamina: &'w Stamina,
    pub stat_points: &'w StatPoints,
    pub team: &'w Team,
    pub union_membership: &'w UnionMembership,
}

#[derive(SystemParam)]
pub struct ScriptFunctionContext<'w, 's> {
    pub query_quest: Query<'w, 's, &'static mut QuestState>,
    pub query_client_entity: Query<'w, 's, &'static ClientEntity>,
    pub query_character: Query<'w, 's, ScriptCharacterQuery<'static>, With<PlayerCharacter>>,
    pub query_player_items:
        Query<'w, 's, (&'static Equipment, &'static Inventory), With<PlayerCharacter>>,
    pub chatbox_events: EventWriter<'w, 's, ChatboxEvent>,
}
