use bevy::{
    hierarchy::BuildChildren,
    prelude::{
        AssetServer, Assets, Commands, Entity, EventReader, EventWriter, Query, Res, ResMut, With,
    },
};
use rose_data::ItemType;

use crate::{
    components::PlayerCharacter,
    effect_loader::spawn_effect,
    events::{ChatboxEvent, ClientEntityEvent},
    render::{EffectMeshMaterial, ParticleMaterial},
    resources::{ClientEntityList, GameData},
    VfsResource,
};

pub fn client_entity_event_system(
    mut commands: Commands,
    mut client_entity_events: EventReader<ClientEntityEvent>,
    mut chatbox_events: EventWriter<ChatboxEvent>,
    query_player: Query<Entity, With<PlayerCharacter>>,
    asset_server: Res<AssetServer>,
    client_entity_list: Res<ClientEntityList>,
    game_data: Res<GameData>,
    vfs_resource: Res<VfsResource>,
    mut effect_mesh_materials: ResMut<Assets<EffectMeshMaterial>>,
    mut particle_materials: ResMut<Assets<ParticleMaterial>>,
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

                    if let Some(effect_entity) = spawn_effect(
                        &vfs_resource.vfs,
                        &mut commands,
                        &asset_server,
                        &mut particle_materials,
                        &mut effect_mesh_materials,
                        "3DDATA/EFFECT/LEVELUP_01.EFT".into(),
                    ) {
                        commands.entity(entity).add_child(effect_entity);
                    }
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
                        if let Some(effect_id) = consumable_item_data.effect_id {
                            if let Some(effect_path) =
                                game_data.effect_database.get_effect(effect_id)
                            {
                                if let Some(effect_entity) = spawn_effect(
                                    &vfs_resource.vfs,
                                    &mut commands,
                                    &asset_server,
                                    &mut particle_materials,
                                    &mut effect_mesh_materials,
                                    effect_path.into(),
                                ) {
                                    commands.entity(entity).add_child(effect_entity);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
