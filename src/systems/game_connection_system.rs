use bevy::{
    math::Vec3,
    prelude::{
        AssetServer, Assets, BuildChildren, Commands, ComputedVisibility, GlobalTransform, Res,
        ResMut, State, Transform, Visibility,
    },
};
use rose_game_common::messages::server::ServerMessage;
use rose_network_common::ConnectionError;

use crate::{
    character_model::{spawn_character_model, CharacterModelList},
    components::PlayerCharacter,
    render::StaticMeshMaterial,
    resources::{AppState, GameConnection, LoadedZone},
};

pub fn game_connection_system(
    mut commands: Commands,
    game_connection: Option<Res<GameConnection>>,
    mut loaded_zone: ResMut<LoadedZone>,
    mut app_state: ResMut<State<AppState>>,
    asset_server: Res<AssetServer>,
    character_model_list: Res<CharacterModelList>,
    mut static_mesh_materials: ResMut<Assets<StaticMeshMaterial>>,
) {
    if game_connection.is_none() {
        return;
    }

    let game_connection = game_connection.unwrap();
    let result: Result<(), anyhow::Error> = loop {
        match game_connection.server_message_rx.try_recv() {
            Ok(ServerMessage::ConnectionResponse(response)) => match response {
                Ok(_) => {}
                Err(_) => {
                    break Err(ConnectionError::ConnectionLost.into());
                }
            },
            Ok(ServerMessage::CharacterData(character_data)) => {
                // Load next zone
                loaded_zone.next_zone_id = Some(character_data.position.zone_id);

                // Spawn character
                let character_model = spawn_character_model(
                    &mut commands,
                    &asset_server,
                    &mut static_mesh_materials,
                    &character_model_list,
                    &character_data.character_info,
                    &character_data.equipment,
                    None,
                );
                let root_bone = character_model.skeleton.bones[0];

                commands
                    .spawn_bundle((
                        character_data.character_info,
                        character_data.basic_stats,
                        character_data.level,
                        character_data.equipment,
                        character_data.experience_points,
                        character_data.skill_list,
                        character_data.hotbar,
                        character_data.health_points,
                        character_data.mana_points,
                        character_data.stat_points,
                        character_data.skill_points,
                        character_data.union_membership,
                        character_data.stamina,
                    ))
                    .insert_bundle((
                        PlayerCharacter {},
                        character_model,
                        Transform::from_xyz(
                            character_data.position.position.x / 100.0,
                            10.0,
                            -character_data.position.position.y / 100.0,
                        ),
                        GlobalTransform::default(),
                        Visibility::default(),
                        ComputedVisibility::default(),
                    ))
                    .add_child(root_bone);

                // Transition to in game state
                app_state.set(AppState::Game).ok();
            }
            Ok(message) => {
                log::warn!("Received unexpected game server message: {:#?}", message);
            }
            Err(crossbeam_channel::TryRecvError::Disconnected) => {
                break Err(ConnectionError::ConnectionLost.into());
            }
            Err(crossbeam_channel::TryRecvError::Empty) => break Ok(()),
        }
    };

    if let Err(error) = result {
        // TODO: Store error somewhere to display to user
        log::warn!("Game server connection error: {}", error);
        commands.remove_resource::<GameConnection>();
    }
}
