use bevy::{
    app::AppExit,
    math::Vec3,
    prelude::{
        AssetServer, Assets, Camera3d, Commands, Entity, EventWriter, FromWorld, Query, Res,
        ResMut, Transform, With, World,
    },
    window::Windows,
};
use bevy_egui::{egui, EguiContext};

use rose_data::ZoneId;
use rose_game_common::messages::client::{ClientMessage, JoinServer};

use crate::{
    components::ActiveMotion,
    events::LoadZoneEvent,
    free_camera::FreeCamera,
    orbit_camera::OrbitCamera,
    resources::{
        Account, LoginConnection, NetworkThread, ServerConfiguration, ServerList, UiResources,
    },
    ui::{Dialog, DialogDataBindings, GetWidget, Widget},
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
    remember_details: bool,
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
            remember_details: false,
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
            .remove::<FreeCamera>()
            .remove::<OrbitCamera>()
            .insert(ActiveMotion::new_repeating(
                asset_server.load("3DDATA/TITLE/CAMERA01_INTRO01.ZMO"),
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

const IID_EDIT_ID: i32 = 2;
const IID_EDIT_PWD: i32 = 3;
const IID_BTN_OK: i32 = 4;
const IID_BTN_CANCEL: i32 = 5;
const IID_CHECKBOX_SAVE_LASTCONECTID: i32 = 10;

#[allow(clippy::too_many_arguments)]
pub fn login_system(
    mut commands: Commands,
    mut ui_state: ResMut<Login>,
    mut egui_context: ResMut<EguiContext>,
    login_connection: Option<Res<LoginConnection>>,
    server_list: Option<Res<ServerList>>,
    network_thread: Res<NetworkThread>,
    ui_resources: Res<UiResources>,
    dialog_assets: Res<Assets<Dialog>>,
    mut exit_events: EventWriter<AppExit>,
) {
    let mut ui_state = &mut *ui_state;
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
            let dialog = if let Some(dialog) = dialog_assets.get(&ui_resources.dialog_login) {
                dialog
            } else {
                return;
            };

            let mut response_username = None;
            let mut response_password = None;
            let mut response_ok = None;
            let mut response_cancel = None;
            let mut enter_pressed = false;

            egui::Window::new("Login")
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .frame(egui::Frame::none())
                .title_bar(false)
                .resizable(false)
                .default_width(dialog.width)
                .default_height(dialog.height)
                .show(egui_context.ctx_mut(), |ui| {
                    dialog.draw(
                        ui,
                        DialogDataBindings {
                            checked: &mut [(
                                IID_CHECKBOX_SAVE_LASTCONECTID,
                                &mut ui_state.remember_details,
                            )],
                            text: &mut [
                                (IID_EDIT_ID, &mut ui_state.username),
                                (IID_EDIT_PWD, &mut ui_state.password),
                            ],
                            response: &mut [
                                (IID_EDIT_ID, &mut response_username),
                                (IID_EDIT_PWD, &mut response_password),
                                (IID_BTN_OK, &mut response_ok),
                                (IID_BTN_CANCEL, &mut response_cancel),
                            ],
                            ..Default::default()
                        },
                        |ui, _| {
                            enter_pressed = ui.input().key_pressed(egui::Key::Enter);
                        },
                    )
                });

            if !ui_state.initial_focus_set {
                if let Some(r) = response_username.as_ref() {
                    r.request_focus();
                }
                ui_state.initial_focus_set = true;
            }

            if ui_state.auto_login || enter_pressed || response_ok.map_or(false, |r| r.clicked()) {
                if ui_state.username.is_empty() {
                    if let Some(r) = response_username.as_ref() {
                        r.request_focus();
                    }
                } else if ui_state.password.is_empty() {
                    if let Some(r) = response_password.as_ref() {
                        r.request_focus();
                    }
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

            if response_cancel.map_or(false, |r| r.clicked()) {
                exit_events.send(AppExit);
            }
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
            let dialog = if let Some(dialog) = dialog_assets.get(&ui_resources.dialog_select_server)
            {
                dialog
            } else {
                return;
            };

            let mut response_ok_button = None;
            let mut response_cancel_button = None;
            let mut try_select_server = ui_state.auto_login;

            egui::Window::new("Select Server")
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .frame(egui::Frame::none())
                .title_bar(false)
                .resizable(false)
                .default_width(dialog.width)
                .default_height(dialog.height)
                .show(egui_context.ctx_mut(), |ui| {
                    dialog.draw(
                        ui,
                        DialogDataBindings {
                            response: &mut [
                                (10, &mut response_ok_button),
                                (11, &mut response_cancel_button),
                            ],
                            ..Default::default()
                        },
                        |ui, _| {
                            let server_list = server_list.as_ref().unwrap();
                            let mut selected_world_server_id = ui_state.selected_world_server_id;
                            let mut selected_game_server_id = ui_state.selected_game_server_id;

                            try_select_server =
                                try_select_server || ui.input().key_pressed(egui::Key::Enter);

                            if let Some(Widget::Listbox(listbox)) = dialog.get_widget(2) {
                                let listbox_rect = listbox.widget_rect(ui.min_rect().min);

                                ui.allocate_ui_at_rect(listbox_rect, |ui| {
                                    ui.vertical(|ui| {
                                        for world_server in server_list.world_servers.iter() {
                                            ui.selectable_value(
                                                &mut selected_world_server_id,
                                                world_server.id,
                                                &world_server.name,
                                            );
                                        }
                                    });
                                });
                            }

                            if let Some(Widget::Listbox(listbox)) = dialog.get_widget(3) {
                                let listbox_rect = listbox.widget_rect(ui.min_rect().min);

                                ui.allocate_ui_at_rect(listbox_rect, |ui| {
                                    ui.vertical(|ui| {
                                        for world_server in server_list.world_servers.iter() {
                                            if world_server.id == selected_world_server_id {
                                                for game_server in world_server.game_servers.iter()
                                                {
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
                                });
                            }
                            ui_state.selected_world_server_id = selected_world_server_id;
                            ui_state.selected_game_server_id = selected_game_server_id;
                        },
                    );
                });

            if response_ok_button.map_or(false, |r| r.clicked()) {
                try_select_server = true;
            }
            if response_cancel_button.map_or(false, |r| r.clicked()) {
                try_select_server = false;
                commands.remove_resource::<LoginConnection>();
            }

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
