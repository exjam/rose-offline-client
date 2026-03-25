use bevy::prelude::{
    EventReader, EventWriter, GlobalTransform, Query, Res,
};

use rose_file_readers::VfsPathBuf;
use rose_game_common::components::Npc;

use crate::{
    components::PlayerCharacter,
    events::{ChatboxEvent, ClientEntityEvent, SpawnEffectData, SpawnEffectEvent},
    resources::GameData,
};

pub fn client_entity_event_system(
    mut client_entity_events: EventReader<ClientEntityEvent>,
    mut chatbox_events: EventWriter<ChatboxEvent>,
    mut spawn_effect_events: EventWriter<SpawnEffectEvent>,
    query_player: Query<&PlayerCharacter>,
    query_npc: Query<(&Npc, &GlobalTransform)>,
    game_data: Res<GameData>,
) {
    let is_player = |entity| query_player.contains(entity);

    for event in client_entity_events.iter() {
        match *event {
            ClientEntityEvent::Die(entity) => {
                if let Ok((npc, _)) = query_npc.get(entity) {
                    if let Some(npc_data) = game_data.npcs.get_npc(npc.id) {
                        if let Some(die_effect_file_id) = npc_data.die_effect_file_id {
                            spawn_effect_events.send(SpawnEffectEvent::OnEntity(
                                entity,
                                None,
                                SpawnEffectData::with_file_id(die_effect_file_id),
                            ));
                        }
                    }
                }
            }
            ClientEntityEvent::LevelUp(entity, level) => {
                if is_player(entity) {
                    if let Some(level) = level {
                        chatbox_events.send(ChatboxEvent::System(format!(
                            "Congratulations! You are now level {}!",
                            level
                        )));
                    }
                };

                spawn_effect_events.send(SpawnEffectEvent::OnEntity(
                    entity,
                    None,
                    SpawnEffectData::with_path(VfsPathBuf::new("3DDATA/EFFECT/LEVELUP_01.EFT")),
                ));
            }
        }
    }
}
