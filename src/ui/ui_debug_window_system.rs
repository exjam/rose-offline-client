use bevy::{
    input::Input,
    math::{EulerRot, Vec3},
    prelude::{
        Camera3d, Commands, Entity, KeyCode, Local, Query, Res, ResMut, Resource, State, Transform,
        With,
    },
};
use bevy_egui::{egui, EguiContext};
use rose_game_common::messages::client::ClientMessage;

use crate::{
    components::PlayerCharacter,
    free_camera::FreeCamera,
    orbit_camera::OrbitCamera,
    resources::{AppState, DebugInspector, GameConnection, WorldConnection},
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DebugCameraType {
    Orbit,
    Free,
}

impl Default for DebugCameraType {
    fn default() -> Self {
        Self::Orbit
    }
}

#[derive(Default, Resource)]
pub struct UiStateDebugWindows {
    pub debug_ui_open: bool,

    pub camera_info_open: bool,
    pub client_entity_list_open: bool,
    pub command_viewer_open: bool,
    pub debug_render_open: bool,
    pub dialog_list_open: bool,
    pub effect_list_open: bool,
    pub item_list_open: bool,
    pub npc_list_open: bool,
    pub object_inspector_open: bool,
    pub physics_open: bool,
    pub skill_list_open: bool,
    pub zone_list_open: bool,
    pub zone_lighting_open: bool,
    pub zone_time_open: bool,
}

#[derive(Default)]
pub struct UiStateDebugMenu {
    selected_camera_type: DebugCameraType,
}

#[allow(clippy::too_many_arguments)]
pub fn ui_debug_menu_system(
    mut commands: Commands,
    mut egui_context: ResMut<EguiContext>,
    mut ui_state_debug_windows: ResMut<UiStateDebugWindows>,
    mut ui_state_debug_menu: Local<UiStateDebugMenu>,
    query_cameras: Query<(Entity, &Transform), With<Camera3d>>,
    query_player: Query<Entity, With<PlayerCharacter>>,
    game_connection: Option<Res<GameConnection>>,
    world_connection: Option<Res<WorldConnection>>,
    keyboard: Res<Input<KeyCode>>,
    mut debug_inspector: ResMut<DebugInspector>,
    mut app_state: ResMut<State<AppState>>,
) {
    if keyboard.pressed(KeyCode::LControl) && keyboard.just_pressed(KeyCode::D) {
        ui_state_debug_windows.debug_ui_open = !ui_state_debug_windows.debug_ui_open;
    }

    if !ui_state_debug_windows.debug_ui_open {
        return;
    }

    let ctx = egui_context.ctx_mut();
    egui::TopBottomPanel::top("ui_debug_menu").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            let player_entity = query_player.get_single().ok();

            ui.menu_button("App", |ui| {
                if ui.button("Model Viewer").clicked() {
                    app_state.set(AppState::ModelViewer).ok();
                }

                if ui.button("Zone Viewer").clicked() {
                    app_state.set(AppState::ZoneViewer).ok();
                }

                ui.separator();

                ui.add_enabled_ui(
                    world_connection.is_none() && game_connection.is_none(),
                    |ui| {
                        if ui.button("Game Login").clicked() {
                            app_state.set(AppState::GameLogin).ok();
                        }
                    },
                );

                ui.add_enabled_ui(
                    world_connection.is_some() && game_connection.is_none(),
                    |ui| {
                        if ui.button("Game Character Select").clicked() {
                            app_state.set(AppState::GameCharacterSelect).ok();
                        }
                    },
                );

                ui.add_enabled_ui(game_connection.is_some(), |ui| {
                    if ui.button("Game").clicked() {
                        app_state.set(AppState::Game).ok();
                    }
                });

                ui.set_enabled(true);
            });

            ui.menu_button("Camera", |ui| {
                let previous_camera_type = ui_state_debug_menu.selected_camera_type;

                if player_entity.is_some() {
                    ui.selectable_value(
                        &mut ui_state_debug_menu.selected_camera_type,
                        DebugCameraType::Orbit,
                        "Orbit",
                    );
                }

                ui.selectable_value(
                    &mut ui_state_debug_menu.selected_camera_type,
                    DebugCameraType::Free,
                    "Free",
                );

                if ui_state_debug_menu.selected_camera_type != previous_camera_type {
                    for (camera_entity, camera_transform) in query_cameras.iter() {
                        match ui_state_debug_menu.selected_camera_type {
                            DebugCameraType::Orbit => {
                                if let Some(player_entity) = player_entity {
                                    commands
                                        .entity(camera_entity)
                                        .remove::<FreeCamera>()
                                        .insert(OrbitCamera::new(
                                            player_entity,
                                            Vec3::new(0.0, 1.7, 0.0),
                                            17.0,
                                        ));
                                }
                            }
                            DebugCameraType::Free => {
                                let (yaw, pitch, _roll) =
                                    camera_transform.rotation.to_euler(EulerRot::YXZ);

                                commands
                                    .entity(camera_entity)
                                    .remove::<OrbitCamera>()
                                    .insert(FreeCamera::new(
                                        camera_transform.translation,
                                        yaw.to_degrees(),
                                        pitch.to_degrees(),
                                    ));
                            }
                        }
                    }
                }
            });

            ui.menu_button("Cheats", |ui| {
                if ui.button("Move Speed 4000").clicked() {
                    if let Some(game_connection) = game_connection.as_ref() {
                        game_connection
                            .client_message_tx
                            .send(ClientMessage::Chat("/speed 4000".to_string()))
                            .ok();
                    }
                }
            });

            ui.menu_button("View", |ui| {
                ui.checkbox(
                    &mut ui_state_debug_windows.command_viewer_open,
                    "Command Viewer",
                );
                ui.checkbox(
                    &mut ui_state_debug_windows.debug_render_open,
                    "Debug Render",
                );
                ui.checkbox(&mut ui_state_debug_windows.dialog_list_open, "Dialog List");
                ui.checkbox(&mut ui_state_debug_windows.effect_list_open, "Effect List");
                ui.checkbox(&mut ui_state_debug_windows.item_list_open, "Item List");
                ui.checkbox(&mut ui_state_debug_windows.npc_list_open, "NPC List");
                ui.checkbox(&mut ui_state_debug_windows.skill_list_open, "Skill List");
                ui.checkbox(&mut ui_state_debug_windows.zone_list_open, "Zone List");
                ui.checkbox(
                    &mut ui_state_debug_windows.zone_lighting_open,
                    "Zone Lighting",
                );
                ui.checkbox(&mut ui_state_debug_windows.zone_time_open, "Zone Time");
                ui.checkbox(
                    &mut ui_state_debug_windows.client_entity_list_open,
                    "Client Entity List",
                );

                if ui
                    .checkbox(
                        &mut ui_state_debug_windows.object_inspector_open,
                        "Object Inspector",
                    )
                    .clicked()
                {
                    if ui_state_debug_windows.object_inspector_open {
                        debug_inspector.enable_picking = true;

                        if let Some(player_entity) = player_entity {
                            debug_inspector.entity = Some(player_entity);
                        }
                    } else {
                        debug_inspector.enable_picking = false;
                    }
                }

                ui.checkbox(&mut ui_state_debug_windows.camera_info_open, "Camera Info");
                ui.checkbox(&mut ui_state_debug_windows.physics_open, "Physics");
            });
        });
    });
}
