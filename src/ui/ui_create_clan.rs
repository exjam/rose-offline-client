use std::num::NonZeroU16;

use bevy::prelude::{Assets, EventReader, EventWriter, Local, Res, ResMut};
use bevy_egui::{egui, EguiContexts};
use rose_game_common::{components::ClanMark, messages::client::ClientMessage};

use crate::{
    events::{ClanDialogEvent, MessageBoxEvent},
    resources::{GameConnection, GameData, UiResources, UiSpriteSheetType},
    ui::{
        widgets::{DataBindings, Dialog, Widget},
        UiSoundEvent, UiStateWindows,
    },
};

const IID_BTN_CONFIRM: i32 = 10;
const IID_BTN_CLOSE: i32 = 11;
const IID_BTN_CANCEL: i32 = 12;
const IID_EDIT_TITLE: i32 = 20;
const IID_EDIT_SLOGAN: i32 = 21;
const IID_TABLE_CLANCENTER: i32 = 30;
const IID_TABLE_CLANBACK: i32 = 40;

pub struct UiCreateClanState {
    pub was_open: bool,
    pub clan_name: String,
    pub clan_slogan: String,
    pub selected_mark_foreground: i32,
    pub selected_mark_background: i32,
    pub scroll_mark_foreground: i32,
    pub scroll_mark_background: i32,
}

impl Default for UiCreateClanState {
    fn default() -> Self {
        Self {
            was_open: false,
            clan_name: String::new(),
            clan_slogan: String::new(),
            selected_mark_foreground: 1,
            selected_mark_background: 1,
            scroll_mark_foreground: 0,
            scroll_mark_background: 0,
        }
    }
}

pub fn ui_create_clan_system(
    mut ui_state: Local<UiCreateClanState>,
    mut ui_state_windows: ResMut<UiStateWindows>,
    mut ui_sound_events: EventWriter<UiSoundEvent>,
    mut egui_context: EguiContexts,
    ui_resources: Res<UiResources>,
    dialog_assets: Res<Assets<Dialog>>,
    mut clan_dialog_events: EventReader<ClanDialogEvent>,
    mut message_box_events: EventWriter<MessageBoxEvent>,
    game_connection: Option<Res<GameConnection>>,
    game_data: Res<GameData>,
) {
    let ui_state = &mut *ui_state;
    let dialog = if let Some(dialog) = dialog_assets.get(&ui_resources.dialog_create_clan) {
        dialog
    } else {
        return;
    };

    for event in clan_dialog_events.iter() {
        let ClanDialogEvent::Open = event;
        ui_state_windows.create_clan_open = true;
    }

    let mut response_confirm_button = None;
    let mut response_cancel_button = None;
    let mut response_close_button = None;

    egui::Window::new("Create Clan")
        .frame(egui::Frame::none())
        .open(&mut ui_state_windows.create_clan_open)
        .title_bar(false)
        .resizable(false)
        .default_width(dialog.width)
        .default_height(dialog.height)
        .show(egui_context.ctx_mut(), |ui| {
            let num_background_sprites = ui_resources.sprite_sheets
                [UiSpriteSheetType::ClanMarkBackground]
                .as_ref()
                .map_or(0, |x| x.sprites.len());
            let num_foreground_sprites = ui_resources.sprite_sheets
                [UiSpriteSheetType::ClanMarkForeground]
                .as_ref()
                .map_or(0, |x| x.sprites.len());

            let (background_extent, background_column_count) =
                if let Some(Widget::Table(table)) = dialog.get_widget(IID_TABLE_CLANBACK) {
                    (table.extent, table.column_count)
                } else {
                    (2, 7)
                };

            let (foreground_extent, foreground_column_count) =
                if let Some(Widget::Table(table)) = dialog.get_widget(IID_TABLE_CLANCENTER) {
                    (table.extent, table.column_count)
                } else {
                    (2, 7)
                };

            let current_background_scroll = ui_state.scroll_mark_background;
            let current_foreground_scroll = ui_state.scroll_mark_foreground;

            let background_rows = (num_background_sprites as i32 + (background_column_count - 1))
                / background_column_count;
            let foreground_rows = (num_foreground_sprites as i32 + (foreground_column_count - 1))
                / foreground_column_count;

            dialog.draw(
                ui,
                DataBindings {
                    sound_events: Some(&mut ui_sound_events),
                    response: &mut [
                        (IID_BTN_CONFIRM, &mut response_confirm_button),
                        (IID_BTN_CLOSE, &mut response_cancel_button),
                        (IID_BTN_CANCEL, &mut response_close_button),
                    ],
                    scroll: &mut [
                        (
                            IID_TABLE_CLANBACK,
                            (
                                &mut ui_state.scroll_mark_background,
                                0..(background_rows + 1),
                                background_extent,
                            ),
                        ),
                        (
                            IID_TABLE_CLANCENTER,
                            (
                                &mut ui_state.scroll_mark_foreground,
                                0..(foreground_rows + 1),
                                foreground_extent,
                            ),
                        ),
                    ],
                    text: &mut [
                        (IID_EDIT_TITLE, &mut ui_state.clan_name),
                        (IID_EDIT_SLOGAN, &mut ui_state.clan_slogan),
                    ],
                    table: &mut [
                        (
                            IID_TABLE_CLANCENTER,
                            (
                                &mut ui_state.selected_mark_foreground,
                                &|ui, index, _x, _y| {
                                    let index =
                                        index + current_foreground_scroll * foreground_column_count;
                                    let (rect, response) = ui.allocate_exact_size(
                                        egui::vec2(20.0, 20.0),
                                        egui::Sense::click(),
                                    );

                                    if ui.is_rect_visible(rect) {
                                        if let Some(sprite) = ui_resources.get_sprite_by_index(
                                            UiSpriteSheetType::ClanMarkForeground,
                                            index as usize,
                                        ) {
                                            sprite.draw(ui, rect.min);
                                        }
                                    }

                                    response
                                },
                            ),
                        ),
                        (
                            IID_TABLE_CLANBACK,
                            (
                                &mut ui_state.selected_mark_background,
                                &|ui, index, _x, _y| {
                                    let index =
                                        index + current_background_scroll * background_column_count;
                                    let (rect, response) = ui.allocate_exact_size(
                                        egui::vec2(20.0, 20.0),
                                        egui::Sense::click(),
                                    );

                                    if ui.is_rect_visible(rect) {
                                        if let Some(sprite) = ui_resources.get_sprite_by_index(
                                            UiSpriteSheetType::ClanMarkBackground,
                                            index as usize,
                                        ) {
                                            sprite.draw(ui, rect.min);
                                        }
                                    }

                                    response
                                },
                            ),
                        ),
                    ],
                    ..Default::default()
                },
                |ui, bindings| {
                    let min = ui.min_rect().min;
                    let selected_mark_background = bindings
                        .get_table(IID_TABLE_CLANBACK)
                        .map_or(0, |(a, _)| *a);
                    let selected_mark_foreground = bindings
                        .get_table(IID_TABLE_CLANCENTER)
                        .map_or(0, |(a, _)| *a);

                    if let Some(sprite) = ui_resources.get_sprite_by_index(
                        UiSpriteSheetType::ClanMarkBackground,
                        selected_mark_background as usize,
                    ) {
                        sprite.draw(ui, min + egui::vec2(186.0, 162.0));
                    }

                    if let Some(sprite) = ui_resources.get_sprite_by_index(
                        UiSpriteSheetType::ClanMarkForeground,
                        selected_mark_foreground as usize,
                    ) {
                        sprite.draw(ui, min + egui::vec2(186.0, 162.0));
                    }
                },
            );
        });

    if response_confirm_button.map_or(false, |r| r.clicked()) {
        if ui_state.clan_name.is_empty() {
            message_box_events.send(MessageBoxEvent::Show {
                message: game_data.client_strings.invalid_name.into(),
                modal: true,
                ok: None,
                cancel: None,
            });
            return;
        }

        if ui_state.clan_slogan.is_empty() {
            message_box_events.send(MessageBoxEvent::Show {
                message: game_data.client_strings.clan_create_error_slogan.into(),
                modal: true,
                ok: None,
                cancel: None,
            });
            return;
        }

        let (Some(mark_background), Some(mark_foreground)) =
        (
            NonZeroU16::new(ui_state.selected_mark_background as u16),
            NonZeroU16::new(ui_state.selected_mark_foreground as u16)
        ) else {
            message_box_events.send(MessageBoxEvent::Show {
                message: game_data.client_strings.clan_create_error.into(),
                modal: true,
                ok: None,
                cancel: None,
            });
            return;
        };

        if let Some(game_connection) = game_connection {
            game_connection
                .client_message_tx
                .send(ClientMessage::ClanCreate {
                    name: ui_state.clan_name.clone(),
                    description: ui_state.clan_slogan.clone(),
                    mark: ClanMark::Premade {
                        background: mark_background,
                        foreground: mark_foreground,
                    },
                })
                .ok();
        }
    }

    if response_cancel_button.map_or(false, |r| r.clicked())
        || response_close_button.map_or(false, |r| r.clicked())
    {
        ui_state_windows.create_clan_open = false;
    }
}
