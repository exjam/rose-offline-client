use bevy::prelude::{Entity, EventReader, EventWriter, Query, Res, With};

use rose_data::ItemType;
use rose_file_readers::VfsPathBuf;

use crate::{
    components::PlayerCharacter,
    events::{ChatboxEvent, ClientEntityEvent, SpawnEffectData, SpawnEffectEvent},
    resources::{ClientEntityList, GameData},
};

pub fn client_entity_event_system(
    mut client_entity_events: EventReader<ClientEntityEvent>,
    mut chatbox_events: EventWriter<ChatboxEvent>,
    mut spawn_effect_events: EventWriter<SpawnEffectEvent>,
    query_player: Query<Entity, With<PlayerCharacter>>,
    client_entity_list: Res<ClientEntityList>,
    game_data: Res<GameData>,
) {
    let player_entity = query_player.single();

    for event in client_entity_events.iter() {
        match *event {
            ClientEntityEvent::LevelUp(client_entity_id, level) => {
                if let Some(entity) = client_entity_list.get(client_entity_id) {
                    if entity == player_entity {
                        chatbox_events.send(ChatboxEvent::System(format!(
                            "Congratulations! You are now level {}!",
                            level
                        )));
                    }

                    spawn_effect_events.send(SpawnEffectEvent::OnEntity(
                        entity,
                        None,
                        SpawnEffectData::with_path(VfsPathBuf::new("3DDATA/EFFECT/LEVELUP_01.EFT")),
                    ));
                }
            }
            ClientEntityEvent::UseItem(client_entity_id, item) => {
                if let Some(entity) = client_entity_list.get(client_entity_id) {
                    if item.item_type != ItemType::Consumable {
                        continue;
                    }

                    if let Some(consumable_item_data) =
                        game_data.items.get_consumable_item(item.item_number)
                    {
                        if let Some(effect_file_id) = consumable_item_data.effect_file_id {
                            spawn_effect_events.send(SpawnEffectEvent::OnEntity(
                                entity,
                                None,
                                SpawnEffectData::with_file_id(effect_file_id),
                            ));
                        }
                    }
                }
            }
        }
    }
}
