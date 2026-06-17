use crate::{
    resources::{GameConnection, UiResources},
    ui::{
        widgets::{DataBindings, Dialog},
        UiSoundEvent, UiStateWindows,
    },
};
use bevy::{
    app::AppExit,
    prelude::{Assets, EventWriter, Local, Res, ResMut},
};
use bevy_egui::{egui, EguiContexts};
use rose_game_common::messages::client::ClientMessage;

const IID_BTN_EXIT: i32 = 10;
const IID_BTN_BACK: i32 = 11;
const IID_BTN_CHAR_SELECT: i32 = 12;
const IID_BTN_CLOSE: i32 = 13;

pub struct ExitState {
    pub pending_exit: bool,
    pub pending_char_select: bool,
}

impl Default for ExitState {
    fn default() -> Self {
        Self {
            pending_exit: false,
            pending_char_select: false,
        }
    }
}

pub fn ui_exit_system(
    mut ui_state: Local<ExitState>,
    mut egui_context: EguiContexts,
    mut ui_state_windows: ResMut<UiStateWindows>,
    mut ui_sound_events: EventWriter<UiSoundEvent>,
    mut exit_events: EventWriter<AppExit>,
    ui_resources: Res<UiResources>,
    dialog_assets: Res<Assets<Dialog>>,
    game_connection: Option<Res<GameConnection>>,
) {
    let ui_state = &mut *ui_state;
    let dialog = if let Some(dialog) = dialog_assets.get(&ui_resources.dialog_exit) {
        dialog
    } else {
        return;
    };

    let mut response_close_button = None;
    let mut response_back_button = None;
    let mut response_char_select_button = None;
    let mut response_exit_button = None;

    egui::Window::new("System Exit")
        .frame(egui::Frame::none())
        .open(&mut ui_state_windows.exit_open)
        .title_bar(false)
        .resizable(false)
        .default_width(dialog.width)
        .default_height(dialog.height)
        .show(egui_context.ctx_mut(), |ui| {
            dialog.draw(
                ui,
                DataBindings {
                    sound_events: Some(&mut ui_sound_events),
                    response: &mut [
                        (IID_BTN_CLOSE, &mut response_close_button),
                        (IID_BTN_BACK, &mut response_back_button),
                        (IID_BTN_CHAR_SELECT, &mut response_char_select_button),
                        (IID_BTN_EXIT, &mut response_exit_button),
                    ],
                    ..Default::default()
                },
                |_, _| {},
            );
        });

    if response_close_button.map_or(false, |r| r.clicked())
        || response_back_button.map_or(false, |r| r.clicked())
    {
        ui_state_windows.exit_open = false;
    }

    if response_char_select_button.map_or(false, |r| r.clicked()) {
        ui_state_windows.exit_open = false;
        ui_state.pending_char_select = true;

        if let Some(game_connection) = game_connection.as_ref() {
            game_connection
                .client_message_tx
                .send(ClientMessage::ReturnToCharacterSelect)
                .ok();
        }
    }

    if response_exit_button.map_or(false, |r| r.clicked()) {
        ui_state_windows.exit_open = false;
        ui_state.pending_exit = true;

        if let Some(game_connection) = game_connection.as_ref() {
            game_connection
                .client_message_tx
                .send(ClientMessage::Logout)
                .ok();
        }
    }

    if ui_state.pending_exit || ui_state.pending_char_select {
        egui::Window::new("Disconnecting...")
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .collapsible(false)
            .show(egui_context.ctx_mut(), |ui| {
                ui.label("Logging out");
            });
    }

    if game_connection.is_none() {
        if ui_state.pending_exit {
            ui_state.pending_exit = false;
            exit_events.send(AppExit);
        }

        if ui_state.pending_char_select {
            ui_state.pending_char_select = false;
        }
    }
}
