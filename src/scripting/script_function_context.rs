use bevy::{ecs::system::SystemParam, prelude::Query};

use rose_game_common::components::{
    BasicStats, CharacterInfo, ExperiencePoints, Level, QuestState, UnionMembership,
};

use crate::components::ClientEntity;

#[derive(SystemParam)]
pub struct ScriptFunctionContext<'w, 's> {
    pub query_quest: Query<'w, 's, &'static QuestState>,
    pub query_client_entity: Query<'w, 's, &'static ClientEntity>,
    pub query_character: Query<
        'w,
        's,
        (
            &'static CharacterInfo,
            &'static BasicStats,
            &'static ExperiencePoints,
            &'static Level,
            &'static UnionMembership,
        ),
    >,
}
