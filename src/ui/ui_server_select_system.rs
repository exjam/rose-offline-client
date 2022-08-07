use bevy::prelude::{Assets, Commands, EventWriter, Local, Res, ResMut};
use bevy_egui::{egui, EguiContext};

use crate::{
    events::LoginEvent,
    resources::{LoginConnection, LoginState, ServerList, UiResources},
    ui::widgets::{DataBindings, Dialog, Widget},
};

#[derive(Default)]
pub struct UiStateServerSelect {
    selected_world_server_id: usize,
    selected_game_server_id: usize,
}

#[allow(clippy::too_many_arguments)]
pub fn ui_server_select_system(
    mut commands: Commands,
    mut ui_state: Local<UiStateServerSelect>,
    mut egui_context: ResMut<EguiContext>,
    login_state: Res<LoginState>,
    dialog_assets: Res<Assets<Dialog>>,
    server_list: Option<Res<ServerList>>,
    ui_resources: Res<UiResources>,
    mut login_events: EventWriter<LoginEvent>,
) {
    if !matches!(*login_state, LoginState::ServerSelect) {
        return;
    }

    let ui_state = &mut *ui_state;
    let dialog = if let Some(dialog) = dialog_assets.get(&ui_resources.dialog_select_server) {
        dialog
    } else {
        return;
    };

    let mut response_ok_button = None;
    let mut response_cancel_button = None;
    let mut try_select_server = false;

    let screen_size = egui_context.ctx_mut().input().screen_rect().size();
    let position = egui::pos2(screen_size.x - dialog.width - 60.0, 100.0);

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
                    ],
                    ..Default::default()
                },
                |ui, _| {
                    let server_list = server_list.as_ref().unwrap();

                    // Ensure selected_world_server_id exists, else select first one
                    if !server_list
                        .world_servers
                        .iter()
                        .any(|world_server| world_server.id == ui_state.selected_world_server_id)
                    {
                        if let Some(world_server) = server_list.world_servers.first() {
                            ui_state.selected_world_server_id = world_server.id;
                        }
                    }

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
        login_events.send(LoginEvent::SelectServer {
            server_id: ui_state.selected_world_server_id,
            channel_id: ui_state.selected_game_server_id,
        });
    }
}
