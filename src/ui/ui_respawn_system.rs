use bevy::prelude::{Assets, EventWriter, Query, Res, With};
use bevy_egui::{egui, EguiContexts};
use rose_game_common::messages::client::{ClientMessage, ReviveRequestType};

use crate::{
    components::{Dead, PlayerCharacter},
    resources::{GameConnection, UiResources},
    ui::{
        widgets::{DataBindings, Dialog},
        UiSoundEvent,
    },
};

const IID_BTN_SAVE_POSITION: i32 = 3;
const IID_BTN_REVIVE_POSITION: i32 = 4;

pub fn ui_respawn_system(
    query_player_dead: Query<&Dead, With<PlayerCharacter>>,
    dialog_assets: Res<Assets<Dialog>>,
    ui_resources: Res<UiResources>,
    mut ui_sound_events: EventWriter<UiSoundEvent>,
    mut egui_context: EguiContexts,
    game_connection: Option<Res<GameConnection>>,
) {
    if query_player_dead.is_empty() {
        return;
    }

    let dialog = if let Some(dialog) = dialog_assets.get(&ui_resources.dialog_respawn) {
        dialog
    } else {
        return;
    };

    let screen_size = egui_context
        .ctx_mut()
        .input(|input| input.screen_rect().size());
    let default_x = screen_size.x / 2.0 - dialog.width / 2.0;
    let default_y = screen_size.y / 2.0 - dialog.height / 2.0;

    let mut response_save_position = None;
    let mut response_revive_position = None;

    egui::Window::new("Respawn")
        .frame(egui::Frame::none())
        .title_bar(false)
        .resizable(false)
        .default_width(dialog.width)
        .default_height(dialog.height)
        .default_pos([default_x, default_y])
        .show(egui_context.ctx_mut(), |ui| {
            dialog.draw(
                ui,
                DataBindings {
                    sound_events: Some(&mut ui_sound_events),
                    response: &mut [
                        (IID_BTN_SAVE_POSITION, &mut response_save_position),
                        (IID_BTN_REVIVE_POSITION, &mut response_revive_position),
                    ],
                    ..Default::default()
                },
                |_, _| {},
            )
        });

    if response_save_position.map_or(false, |x| x.clicked()) {
        if let Some(game_connection) = game_connection.as_ref() {
            game_connection
                .client_message_tx
                .send(ClientMessage::ReviveRequest(
                    ReviveRequestType::SavePosition,
                ))
                .ok();
        }
    }

    if response_revive_position.map_or(false, |x| x.clicked()) {
        if let Some(game_connection) = game_connection.as_ref() {
            game_connection
                .client_message_tx
                .send(ClientMessage::ReviveRequest(
                    ReviveRequestType::RevivePosition,
                ))
                .ok();
        }
    }
}
