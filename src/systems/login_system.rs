use bevy::{
    core::Time,
    math::Vec3,
    prelude::{
        AssetServer, Commands, Entity, EventWriter, FromWorld, Query, Res, ResMut, Transform, With,
        World,
    },
    render::camera::Camera3d,
    window::Windows,
};
use bevy_egui::{egui, EguiContext};

use rose_data::ZoneId;
use rose_game_common::messages::client::{ClientMessage, JoinServer};

use crate::{
    components::ActiveMotion,
    events::LoadZoneEvent,
    fly_camera::FlyCameraController,
    follow_camera::FollowCameraController,
    resources::{Account, LoginConnection, NetworkThread, ServerConfiguration, ServerList},
};

enum LoginState {
    Input,
    WaitServerList,
    ServerSelect,
    JoiningServer,
}

impl Default for LoginState {
    fn default() -> Self {
        Self::Input
    }
}

pub struct Login {
    state: LoginState,
    initial_focus_set: bool,
    ip: String,
    port: String,
    username: String,
    password: String,
    selected_world_server_id: usize,
    selected_game_server_id: usize,
    auto_login: bool,
}

impl FromWorld for Login {
    fn from_world(world: &mut World) -> Self {
        let config = world.get_resource::<ServerConfiguration>().unwrap();

        Self {
            state: LoginState::default(),
            initial_focus_set: false,
            ip: config.ip.clone(),
            port: config.port.clone(),
            username: config.preset_username.as_ref().cloned().unwrap_or_default(),
            password: config.preset_password.as_ref().cloned().unwrap_or_default(),
            selected_world_server_id: config.preset_server_id.unwrap_or(0),
            selected_game_server_id: config.preset_channel_id.unwrap_or(0),
            auto_login: config.auto_login,
        }
    }
}

pub fn login_state_enter_system(
    mut commands: Commands,
    mut loaded_zone: EventWriter<LoadZoneEvent>,
    mut windows: ResMut<Windows>,
    query_cameras: Query<Entity, With<Camera3d>>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
) {
    // Ensure cursor is not locked
    if let Some(window) = windows.get_primary_mut() {
        window.set_cursor_lock_mode(false);
        window.set_cursor_visibility(true);
    }

    // Reset camera
    for entity in query_cameras.iter() {
        commands
            .entity(entity)
            .insert(
                Transform::from_xyz(5240.0, 10.0, -5400.0)
                    .looking_at(Vec3::new(5200.0, 35.0, -5300.0), Vec3::Y),
            )
            .remove::<FlyCameraController>()
            .remove::<FollowCameraController>()
            .insert(ActiveMotion::new_repeating(
                asset_server.load("3DDATA/TITLE/CAMERA01_INTRO01.ZMO"),
                time.seconds_since_startup(),
            ));
    }

    commands.remove_resource::<Account>();
    commands.init_resource::<Login>();

    loaded_zone.send(LoadZoneEvent::new(ZoneId::new(4).unwrap()));
}

pub fn login_state_exit_system(mut commands: Commands) {
    commands.remove_resource::<LoginConnection>();
    commands.remove_resource::<Login>();
}

#[allow(clippy::too_many_arguments)]
pub fn login_system(
    mut commands: Commands,
    mut ui_state: ResMut<Login>,
    mut egui_context: ResMut<EguiContext>,
    login_connection: Option<Res<LoginConnection>>,
    server_list: Option<Res<ServerList>>,
    network_thread: Res<NetworkThread>,
) {
    if login_connection.is_none() {
        // If we have no connection, return to input state
        ui_state.state = LoginState::Input;
    }

    if server_list.is_some() {
        match ui_state.state {
            LoginState::Input => {
                // We must have disconnected, remove the old server list
                commands.remove_resource::<ServerList>();
            }
            LoginState::WaitServerList => {
                // We have server list, transition to select
                ui_state.state = LoginState::ServerSelect;
            }
            _ => {}
        }
    }

    match ui_state.state {
        LoginState::Input => {
            egui::Window::new("Login")
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .collapsible(false)
                .show(egui_context.ctx_mut(), |ui| {
                    let (text_username, text_password) = egui::Grid::new("login_dialog_grid")
                        .num_columns(2)
                        .show(ui, |ui| {
                            ui.label("Username");
                            let text_username = ui.text_edit_singleline(&mut ui_state.username);
                            ui.end_row();

                            ui.label("Password");
                            let text_password = ui.add(
                                egui::TextEdit::singleline(&mut ui_state.password).password(true),
                            );
                            ui.end_row();

                            if !ui_state.initial_focus_set {
                                text_username.request_focus();
                                ui_state.initial_focus_set = true;
                            }

                            (text_username, text_password)
                        })
                        .inner;

                    ui.separator();

                    let mut try_start_login =
                        ui.input().key_pressed(egui::Key::Enter) || ui_state.auto_login;
                    ui.horizontal(|ui| {
                        if ui.button("Login").clicked() {
                            try_start_login = true;
                        }

                        if ui.button("Exit").clicked() {
                            // take some action here
                        }
                    });

                    if try_start_login {
                        if ui_state.username.is_empty() {
                            text_username.request_focus();
                        } else if ui_state.password.is_empty() {
                            text_password.request_focus();
                        } else {
                            ui_state.state = LoginState::WaitServerList;
                            commands.insert_resource(Account {
                                username: ui_state.username.clone(),
                                password_md5: format!("{:x}", md5::compute(&ui_state.password)),
                            });
                            commands.insert_resource(network_thread.connect_login(
                                &ui_state.ip,
                                ui_state.port.parse::<u16>().unwrap_or(29000),
                            ));
                        }
                    }
                });
        }
        LoginState::WaitServerList => {
            egui::Window::new("Connecting...")
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .collapsible(false)
                .show(egui_context.ctx_mut(), |ui| {
                    ui.label("Logging in");
                });
        }
        LoginState::ServerSelect => {
            egui::Window::new("Select Server")
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .collapsible(false)
                .show(egui_context.ctx_mut(), |ui| {
                    let mut try_select_server =
                        ui.input().key_pressed(egui::Key::Enter) || ui_state.auto_login;
                    let server_list = server_list.as_ref().unwrap();

                    ui.horizontal(|ui| {
                        let mut selected_world_server_id = ui_state.selected_world_server_id;
                        let mut selected_game_server_id = ui_state.selected_game_server_id;

                        ui.vertical(|ui| {
                            ui.label("World Server");
                            for world_server in server_list.world_servers.iter() {
                                ui.selectable_value(
                                    &mut selected_world_server_id,
                                    world_server.id,
                                    &world_server.name,
                                );
                            }
                        });

                        ui.vertical(|ui| {
                            ui.label("Game Server");
                            for world_server in server_list.world_servers.iter() {
                                if world_server.id == selected_world_server_id {
                                    for game_server in world_server.game_servers.iter() {
                                        let response = ui.selectable_value(
                                            &mut selected_game_server_id,
                                            game_server.id,
                                            &game_server.name,
                                        );

                                        if response.double_clicked() {
                                            try_select_server = true;
                                        }
                                    }
                                }
                            }
                        });

                        ui_state.selected_world_server_id = selected_world_server_id;
                        ui_state.selected_world_server_id = selected_world_server_id;
                    });

                    ui.horizontal(|ui| {
                        if ui.button("Play").clicked() {
                            try_select_server = true;
                        }

                        if ui.button("Logout").clicked() {
                            commands.remove_resource::<LoginConnection>();
                            try_select_server = false;
                        }
                    });

                    if try_select_server {
                        if let Some(connection) = login_connection.as_ref() {
                            connection
                                .client_message_tx
                                .send(ClientMessage::JoinServer(JoinServer {
                                    server_id: ui_state.selected_world_server_id,
                                    channel_id: ui_state.selected_game_server_id,
                                }))
                                .ok();
                            ui_state.state = LoginState::JoiningServer;
                        }
                    }
                });
        }
        LoginState::JoiningServer => {
            egui::Window::new("Connecting...")
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .collapsible(false)
                .show(egui_context.ctx_mut(), |ui| {
                    ui.label("Connecting to channel");
                });
        }
    }
}
