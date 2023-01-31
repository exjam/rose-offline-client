use std::time::{Duration, Instant};

use bevy::{
    input::Input,
    math::Vec3,
    prelude::{
        AssetServer, Camera, Camera3d, Commands, Component, ComputedVisibility,
        DespawnRecursiveExt, Entity, EventReader, EventWriter, GlobalTransform, Handle, Local,
        MouseButton, Query, Res, ResMut, Resource, State, Transform, Visibility, With,
    },
    render::{camera::Projection, mesh::skinning::SkinnedMesh},
    window::{CursorGrabMode, Windows},
};
use bevy_egui::{egui, EguiContext};
use bevy_rapier3d::prelude::{InteractionGroups, QueryFilter, RapierContext};

use rose_data::{CharacterMotionAction, ZoneId};
use rose_game_common::messages::{
    client::{ClientMessage, DeleteCharacter, SelectCharacter},
    server::CreateCharacterError,
};

use crate::{
    components::{
        ActiveMotion, CharacterModel, ColliderParent, COLLISION_FILTER_CLICKABLE,
        COLLISION_GROUP_CHARACTER, COLLISION_GROUP_PLAYER,
    },
    events::{CharacterSelectEvent, GameConnectionEvent, LoadZoneEvent, WorldConnectionEvent},
    ray_from_screenspace::ray_from_screenspace,
    resources::{
        AppState, CharacterList, CharacterSelectState, GameData, ServerConfiguration,
        WorldConnection,
    },
    systems::{FreeCamera, OrbitCamera},
    zmo_asset_loader::ZmoAsset,
};

#[derive(Component)]
pub struct CharacterSelectCharacter {
    pub index: usize,
}

#[derive(Resource)]
pub struct CharacterSelectModelList {
    models: Vec<(Option<String>, Entity)>,
    select_motion: Handle<ZmoAsset>,
}

pub fn character_select_enter_system(
    mut commands: Commands,
    mut windows: ResMut<Windows>,
    query_cameras: Query<Entity, With<Camera3d>>,
    asset_server: Res<AssetServer>,
    game_data: Res<GameData>,
) {
    if let Some(window) = windows.get_primary_mut() {
        window.set_cursor_grab_mode(CursorGrabMode::None);
        window.set_cursor_visibility(true);
    }

    // Reset camera
    for entity in query_cameras.iter() {
        commands
            .entity(entity)
            .insert(
                Transform::from_xyz(5200.0, 3.4, -5220.0)
                    .looking_at(Vec3::new(5200.0, 3.4, -5200.0), Vec3::Y),
            )
            .remove::<FreeCamera>()
            .remove::<OrbitCamera>()
            .insert(ActiveMotion::new_once(
                asset_server.load("3DDATA/TITLE/CAMERA01_INSELECT01.ZMO"),
            ));
    }

    // Reset state
    commands.insert_resource(CharacterSelectState::Entering);

    // Spawn entities to use for character list models
    let mut models = Vec::with_capacity(game_data.character_select_positions.len());
    for (index, transform) in game_data.character_select_positions.iter().enumerate() {
        let entity = commands
            .spawn((
                CharacterSelectCharacter { index },
                *transform,
                GlobalTransform::default(),
                Visibility::default(),
                ComputedVisibility::default(),
            ))
            .id();
        models.push((None, entity));
    }
    commands.insert_resource(CharacterSelectModelList {
        models,
        select_motion: asset_server.load("3DDATA/MOTION/AVATAR/EVENT_SELECT_M1.ZMO"),
    });
}

pub fn character_select_exit_system(
    mut commands: Commands,
    model_list: Res<CharacterSelectModelList>,
) {
    // Despawn character models
    for (_, entity) in model_list.models.iter() {
        commands.entity(*entity).despawn_recursive();
    }

    commands.remove_resource::<CharacterList>();
    commands.remove_resource::<CharacterSelectState>();
    commands.remove_resource::<CharacterSelectModelList>();
}

pub fn character_select_models_system(
    mut commands: Commands,
    mut model_list: ResMut<CharacterSelectModelList>,
    character_list: Option<Res<CharacterList>>,
    character_select_state: Res<CharacterSelectState>,
    query_characters: Query<(Option<&ActiveMotion>, &CharacterModel), With<SkinnedMesh>>,
) {
    // Ensure all character list models are up to date
    if let Some(character_list) = character_list.as_ref() {
        for (index, character) in character_list.characters.iter().enumerate() {
            let entity = model_list.models[index].1;

            // If the character list has changed, recreate model
            if model_list.models[index].0.as_ref() != Some(&character.info.name) {
                commands
                    .entity(model_list.models[index].1)
                    .insert((character.info.clone(), character.equipment.clone()));
                model_list.models[index].0 = Some(character.info.name.clone());
            }

            if let Ok((active_motion, character_model)) = query_characters.get(entity) {
                let deleting = character.delete_time.is_some();
                let selected = if let CharacterSelectState::CharacterSelect(Some(selected_index)) =
                    *character_select_state
                {
                    selected_index == index
                } else {
                    false
                };

                let desired_motion = if deleting {
                    &character_model.action_motions[CharacterMotionAction::Sit]
                } else if selected {
                    &model_list.select_motion
                } else {
                    &character_model.action_motions[CharacterMotionAction::Stop1]
                };

                if active_motion.map_or(true, |x| x.motion.id() != desired_motion.id()) {
                    commands
                        .entity(entity)
                        .insert(ActiveMotion::new_repeating(desired_motion.clone()));
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn character_select_system(
    mut commands: Commands,
    mut app_state: ResMut<State<AppState>>,
    mut character_select_state: ResMut<CharacterSelectState>,
    mut egui_context: ResMut<EguiContext>,
    mut game_connection_events: EventReader<GameConnectionEvent>,
    mut world_connection_events: EventReader<WorldConnectionEvent>,
    mut load_zone_events: EventWriter<LoadZoneEvent>,
    mut join_zone_id: Local<Option<ZoneId>>,
    query_camera: Query<(Entity, &Camera, &GlobalTransform, Option<&ActiveMotion>), With<Camera3d>>,
    world_connection: Option<Res<WorldConnection>>,
    mut character_list: Option<ResMut<CharacterList>>,
    server_configuration: Res<ServerConfiguration>,
    asset_server: Res<AssetServer>,
) {
    let character_select_state = &mut *character_select_state;
    let world_connection = if let Some(world_connection) = world_connection {
        world_connection
    } else {
        // Disconnected, return to login
        app_state.set(AppState::GameLogin).ok();
        return;
    };

    for event in world_connection_events.iter() {
        match event {
            WorldConnectionEvent::CreateCharacterResponse(response) => match response {
                Ok(_) => {
                    let (camera_entity, _, _, _) = query_camera.single();
                    commands
                        .entity(camera_entity)
                        .insert(ActiveMotion::new_once(
                            asset_server.load("3DDATA/TITLE/CAMERA01_OUTCREATE01.ZMO"),
                        ));
                    *character_select_state = CharacterSelectState::CharacterSelect(None);

                    world_connection
                        .client_message_tx
                        .send(ClientMessage::GetCharacterList)
                        .ok();
                }
                Err(CreateCharacterError::Failed) => {
                    // TODO: Show modal error dialog with error message
                    // character_select_state.create_character_error_message =
                    //    "Unknown error creating character".into();
                    *character_select_state = CharacterSelectState::CharacterCreate;
                }
                Err(CreateCharacterError::AlreadyExists) => {
                    // TODO: Show modal error dialog with error message
                    // character_select_state.create_character_error_message =
                    //    "Character name already exists".into();
                    *character_select_state = CharacterSelectState::CharacterCreate;
                }
                Err(CreateCharacterError::NoMoreSlots) => {
                    // TODO: Show modal error dialog with error message
                    //character_select_state.create_character_error_message =
                    //    "Cannot create more characters".into();
                    *character_select_state = CharacterSelectState::CharacterCreate;
                }
                Err(CreateCharacterError::InvalidValue) => {
                    // TODO: Show modal error dialog with error message
                    // character_select_state.create_character_error_message = "Invalid value".into();
                    *character_select_state = CharacterSelectState::CharacterCreate;
                }
            },
            WorldConnectionEvent::DeleteCharacterResponse(response) => match response {
                Ok(response) => {
                    if let Some(character_list) = character_list.as_mut() {
                        for character in character_list.characters.iter_mut() {
                            if character.info.name == response.name {
                                character.delete_time = response.delete_time.clone();
                            }
                        }
                    } else {
                        world_connection
                            .client_message_tx
                            .send(ClientMessage::GetCharacterList)
                            .ok();
                    }
                }
                Err(_) => {
                    // TODO: Show delete character error message
                }
            },
        }
    }

    match character_select_state {
        CharacterSelectState::Entering => {
            let (_, _, _, camera_motion) = query_camera.single();
            if camera_motion.is_none() || server_configuration.auto_login {
                *character_select_state = CharacterSelectState::CharacterSelect(None);
            }
        }
        CharacterSelectState::CharacterSelect(_) => {}
        CharacterSelectState::CharacterCreate => {}
        CharacterSelectState::CharacterCreating => {
            egui::Window::new("Creating character...")
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .collapsible(false)
                .show(egui_context.ctx_mut(), |ui| {
                    ui.label("Creating character");
                });
        }
        CharacterSelectState::ConnectingGameServer => {
            egui::Window::new("Connecting...")
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .collapsible(false)
                .show(egui_context.ctx_mut(), |ui| {
                    ui.label("Connecting to game");
                });

            for event in game_connection_events.iter() {
                let &GameConnectionEvent::Connected(zone_id) = event;

                // Start camera animation
                let (camera_entity, _, _, _) = query_camera.single();
                commands
                    .entity(camera_entity)
                    .insert(ActiveMotion::new_once(
                        asset_server.load("3DDATA/TITLE/CAMERA01_INGAME01.ZMO"),
                    ));

                *character_select_state = CharacterSelectState::Leaving;
                *join_zone_id = Some(zone_id);
            }
        }
        CharacterSelectState::Leaving => {
            let (_, _, _, camera_motion) = query_camera.single();
            if camera_motion.is_none() || server_configuration.auto_login {
                // Wait until camera motion complete, then load the zone!
                *character_select_state = CharacterSelectState::Loading;
                load_zone_events.send(LoadZoneEvent::new(join_zone_id.take().unwrap()));
            }
        }
        CharacterSelectState::Loading => {}
    }
}

pub fn character_select_event_system(
    mut commands: Commands,
    mut character_select_state: ResMut<CharacterSelectState>,
    mut character_select_events: EventReader<CharacterSelectEvent>,
    character_list: Option<Res<CharacterList>>,
    world_connection: Option<Res<WorldConnection>>,
) {
    for event in character_select_events.iter() {
        match event {
            CharacterSelectEvent::SelectCharacter(index) => {
                if matches!(
                    *character_select_state,
                    CharacterSelectState::CharacterSelect(_)
                ) {
                    *character_select_state = CharacterSelectState::CharacterSelect(Some(*index));
                }
            }
            CharacterSelectEvent::PlaySelected => {
                if let CharacterSelectState::CharacterSelect(Some(selected_character_index)) =
                    *character_select_state
                {
                    if let Some(character_list) = character_list.as_ref() {
                        if let Some(selected_character) =
                            character_list.characters.get(selected_character_index)
                        {
                            if selected_character.delete_time.is_none() {
                                if let Some(world_connection) = world_connection.as_ref() {
                                    world_connection
                                        .client_message_tx
                                        .send(ClientMessage::SelectCharacter(SelectCharacter {
                                            slot: selected_character_index as u8,
                                            name: selected_character.info.name.clone(),
                                        }))
                                        .ok();
                                }

                                *character_select_state =
                                    CharacterSelectState::ConnectingGameServer;
                            }
                        }
                    }
                }
            }
            CharacterSelectEvent::DeleteSelected => {
                if let CharacterSelectState::CharacterSelect(Some(selected_character_index)) =
                    *character_select_state
                {
                    if let Some(character_list) = character_list.as_ref() {
                        if let Some(selected_character) =
                            character_list.characters.get(selected_character_index)
                        {
                            if let Some(world_connection) = world_connection.as_ref() {
                                world_connection
                                    .client_message_tx
                                    .send(ClientMessage::DeleteCharacter(DeleteCharacter {
                                        slot: selected_character_index as u8,
                                        name: selected_character.info.name.clone(),
                                        is_delete: selected_character.delete_time.is_none(),
                                    }))
                                    .ok();
                            }
                        }
                    }
                }
            }
            CharacterSelectEvent::Disconnect => {
                commands.remove_resource::<WorldConnection>();
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn character_select_input_system(
    mut character_select_state: ResMut<CharacterSelectState>,
    mut egui_ctx: ResMut<EguiContext>,
    mouse_button_input: Res<Input<MouseButton>>,
    rapier_context: Res<RapierContext>,
    windows: Res<Windows>,
    mut last_selected_time: Local<Option<Instant>>,
    query_camera: Query<(&Camera, &Projection, &GlobalTransform), With<Camera3d>>,
    query_collider_parent: Query<&ColliderParent>,
    query_select_character: Query<&CharacterSelectCharacter>,
    mut character_select_events: EventWriter<CharacterSelectEvent>,
) {
    if egui_ctx.ctx_mut().wants_pointer_input() {
        // Mouse is over UI
        return;
    }

    let selected_character_index =
        if let CharacterSelectState::CharacterSelect(selected_character_index) =
            &mut *character_select_state
        {
            selected_character_index
        } else {
            // Not in character select
            return;
        };

    let cursor_position =
        if let Some(cursor_position) = windows.get_primary().and_then(|w| w.cursor_position()) {
            cursor_position
        } else {
            return;
        };

    if mouse_button_input.just_pressed(MouseButton::Left) {
        for (camera, camera_projection, camera_transform) in query_camera.iter() {
            if let Some((ray_origin, ray_direction)) = ray_from_screenspace(
                cursor_position,
                &windows,
                camera,
                camera_projection,
                camera_transform,
            ) {
                if let Some((collider_entity, _)) = rapier_context.cast_ray(
                    ray_origin,
                    ray_direction,
                    10000000.0,
                    false,
                    QueryFilter::new().groups(InteractionGroups::new(
                        bevy_rapier3d::rapier::geometry::Group::from_bits_truncate(
                            COLLISION_FILTER_CLICKABLE,
                        ),
                        bevy_rapier3d::rapier::geometry::Group::from_bits_truncate(
                            COLLISION_GROUP_CHARACTER | COLLISION_GROUP_PLAYER,
                        ),
                    )),
                ) {
                    let hit_entity = query_collider_parent
                        .get(collider_entity)
                        .map_or(collider_entity, |collider_parent| collider_parent.entity);

                    if let Ok(select_character) = query_select_character.get(hit_entity) {
                        let now = Instant::now();

                        if *selected_character_index == Some(select_character.index) {
                            if let Some(last_selected_time) = *last_selected_time {
                                if now - last_selected_time < Duration::from_millis(250) {
                                    character_select_events
                                        .send(CharacterSelectEvent::PlaySelected);
                                }
                            }
                        }

                        *selected_character_index = Some(select_character.index);
                        *last_selected_time = Some(now);
                    }
                }
            }
        }
    }
}
