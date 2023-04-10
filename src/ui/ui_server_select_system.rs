use bevy::prelude::{Assets, Commands, EventWriter, Local, Res};
use bevy_egui::{egui, EguiContexts};

use crate::{
    events::LoginEvent,
    resources::{LoginConnection, LoginState, ServerList, UiResources},
    ui::widgets::{DataBindings, Dialog},
};

#[derive(Default)]
pub struct UiStateServerSelect {
    selected_world_server_index: i32,
    selected_game_server_index: i32,
}

#[allow(clippy::too_many_arguments)]
pub fn ui_server_select_system(
    mut commands: Commands,
    mut ui_state: Local<UiStateServerSelect>,
    mut egui_context: EguiContexts,
    login_state: Res<LoginState>,
    dialog_assets: Res<Assets<Dialog>>,
    server_list: Option<Res<ServerList>>,
    ui_resources: Res<UiResources>,
    mut login_events: EventWriter<LoginEvent>,
) {
    if !matches!(*login_state, LoginState::ServerSelect) {
        return;
    }
    let Some(server_list) = server_list else {
        return;
    };

    let ui_state = &mut *ui_state;
    let dialog = if let Some(dialog) = dialog_assets.get(&ui_resources.dialog_select_server) {
        dialog
    } else {
        return;
    };

    let mut response_ok_button = None;
    let mut response_cancel_button = None;
    let mut response_game_server_listbox = None;
    let mut try_select_server = false;

    let screen_size = egui_context
        .ctx_mut()
        .input(|input| input.screen_rect().size());
    let position = egui::pos2(screen_size.x - dialog.width - 60.0, 100.0);

    let selected_world_server_index = ui_state.selected_world_server_index as usize;

    egui::Window::new("Select Server")
        .frame(egui::Frame::none())
        .title_bar(false)
        .resizable(false)
        .default_width(dialog.width)
        .default_height(dialog.height)
        .fixed_pos(position)
        .show(egui_context.ctx_mut(), |ui| {
            dialog.draw(
                ui,
                DataBindings {
                    response: &mut [
                        (10, &mut response_ok_button),
                        (11, &mut response_cancel_button),
                        (3, &mut response_game_server_listbox),
                    ],
                    listbox: &mut [
                        (
                            2,
                            (
                                &mut ui_state.selected_world_server_index,
                                &|index| -> Option<String> {
                                    server_list
                                        .world_servers
                                        .get(index as usize)
                                        .map(|x| x.name[1..].to_string())
                                },
                            ),
                        ),
                        (
                            3,
                            (
                                &mut ui_state.selected_game_server_index,
                                &|index| -> Option<String> {
                                    server_list
                                        .world_servers
                                        .get(selected_world_server_index)
                                        .and_then(|world_server| {
                                            world_server.game_servers.get(index as usize)
                                        })
                                        .map(|game_server| game_server.name.clone())
                                },
                            ),
                        ),
                    ],
                    ..Default::default()
                },
                |ui, _| {
                    if ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                        try_select_server = true;
                    }
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

    if response_game_server_listbox.map_or(false, |r| r.double_clicked()) {
        try_select_server = true;
    }

    if try_select_server {
        if let Some(world_server) = server_list
            .world_servers
            .get(ui_state.selected_world_server_index as usize)
        {
            if let Some(game_server) = world_server
                .game_servers
                .get(ui_state.selected_game_server_index as usize)
            {
                login_events.send(LoginEvent::SelectServer {
                    server_id: world_server.id,
                    channel_id: game_server.id,
                });
            }
        }
    }
}
