use bevy::prelude::{Added, Changed, Entity, Or, Query, Res, With};

use rose_game_common::{
    components::{CharacterInfo, Level},
    messages::client::ClientMessage,
};

use crate::{
    components::{Clan, PlayerCharacter},
    resources::WorldConnection,
};

pub fn clan_system(
    query_player_clan_added: Query<Entity, (With<PlayerCharacter>, Added<Clan>)>,
    query_player_updated: Query<
        (&CharacterInfo, &Level),
        (
            With<PlayerCharacter>,
            Or<(Changed<CharacterInfo>, Changed<Level>)>,
        ),
    >,
    world_connection: Option<Res<WorldConnection>>,
) {
    if !query_player_clan_added.is_empty() {
        if let Some(world_connection) = world_connection.as_ref() {
            world_connection
                .client_message_tx
                .send(ClientMessage::ClanGetMemberList)
                .ok();
        }
    }

    if let Ok((character_info, &level)) = query_player_updated.get_single() {
        if let Some(world_connection) = world_connection.as_ref() {
            world_connection
                .client_message_tx
                .send(ClientMessage::ClanUpdateCharacterInfo {
                    job: character_info.job,
                    level,
                })
                .ok();
        }
    }
}
