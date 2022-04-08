use bevy::{
    ecs::system::SystemParam,
    prelude::{EventWriter, Query, With},
};

use rose_game_common::components::{
    BasicStats, CharacterInfo, Equipment, ExperiencePoints, Inventory, Level, QuestState,
    UnionMembership,
};

use crate::{
    components::{ClientEntity, PlayerCharacter},
    events::ChatboxEvent,
};

#[derive(SystemParam)]
pub struct ScriptFunctionContext<'w, 's> {
    pub query_quest: Query<'w, 's, &'static mut QuestState>,
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
    pub query_player_items:
        Query<'w, 's, (&'static Equipment, &'static Inventory), With<PlayerCharacter>>,
    pub chatbox_events: EventWriter<'w, 's, ChatboxEvent>,
}
