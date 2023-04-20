use bevy::prelude::{Assets, EventReader, EventWriter, Local, Res};
use bevy_egui::{egui, EguiContexts};

use rose_game_common::messages::client::ClientMessage;

use crate::{
    events::ChatboxEvent,
    resources::{GameConnection, UiResources},
    ui::{
        widgets::{DataBindings, Dialog},
        UiSoundEvent,
    },
};

const MAX_CHATBOX_ENTRIES: usize = 100;

// TODO: Implement the chat filters
// const IID_BTN_FILTER: i32 = 10;
const IID_EDITBOX: i32 = 15;

const IID_CHAT_LIST_IMAGE: i32 = 6;

const IID_LISTBOX_ALL: i32 = 20;
const IID_SCROLLBAR_ALL: i32 = 21;

const IID_LISTBOX_WHISPER: i32 = 25;
const IID_SCROLLBAR_WHISPER: i32 = 26;

const IID_LISTBOX_TRADE: i32 = 30;
const IID_SCROLLBAR_TRADE: i32 = 31;

const IID_LISTBOX_PARTY: i32 = 35;
const IID_SCROLLBAR_PARTY: i32 = 36;

const IID_LISTBOX_CLAN: i32 = 40;
const IID_SCROLLBAR_CLAN: i32 = 41;

const IID_LISTBOX_ALLIED: i32 = 45;
const IID_SCROLLBAR_ALLIED: i32 = 46;

const IID_RADIOBOX: i32 = 50;
const IID_BTN_ALL: i32 = 51;
const IID_BTN_WHISPER: i32 = 52;
const IID_BTN_TRADE: i32 = 53;
const IID_BTN_PARTY: i32 = 54;
const IID_BTN_CLAN: i32 = 55;
const IID_BTN_ALLIED: i32 = 56;

const CHAT_COLOR_TIMESTAMP: egui::Color32 = egui::Color32::from_rgb(150, 150, 150);
const CHAT_COLOR_NORMAL: egui::Color32 = egui::Color32::from_rgb(255, 255, 255);
const CHAT_COLOR_SHOUT: egui::Color32 = egui::Color32::from_rgb(189, 250, 255);
const CHAT_COLOR_WHISPER: egui::Color32 = egui::Color32::from_rgb(201, 255, 144);
const CHAT_COLOR_ANNOUNCE: egui::Color32 = egui::Color32::from_rgb(255, 188, 172);
const CHAT_COLOR_PARTY: egui::Color32 = egui::Color32::from_rgb(255, 237, 140);
const CHAT_COLOR_SYSTEM: egui::Color32 = egui::Color32::from_rgb(255, 224, 229);
const CHAT_COLOR_QUEST: egui::Color32 = egui::Color32::from_rgb(151, 221, 241);
const CHAT_COLOR_ALLIED: egui::Color32 = egui::Color32::from_rgb(255, 228, 122);
const CHAT_COLOR_CLAN: egui::Color32 = egui::Color32::from_rgb(255, 228, 122);

pub struct UiStateChatbox {
    textbox_text: String,
    textbox_layout_job: egui::text::LayoutJob,
    cleanup_layout_text_counter: usize,
    selected_channel: i32,
}

impl Default for UiStateChatbox {
    fn default() -> Self {
        Self {
            textbox_text: Default::default(),
            textbox_layout_job: Default::default(),
            cleanup_layout_text_counter: 0,
            selected_channel: IID_BTN_ALL,
        }
    }
}

pub fn ui_chatbox_system(
    mut egui_context: EguiContexts,
    mut ui_state_chatbox: Local<UiStateChatbox>,
    mut chatbox_events: EventReader<ChatboxEvent>,
    game_connection: Option<Res<GameConnection>>,
    ui_resources: Res<UiResources>,
    mut ui_sound_events: EventWriter<UiSoundEvent>,
    dialog_assets: Res<Assets<Dialog>>,
) {
    let ui_state_chatbox = &mut *ui_state_chatbox;
    let dialog = if let Some(dialog) = dialog_assets.get(&ui_resources.dialog_chatbox) {
        dialog
    } else {
        return;
    };

    let local_time = chrono::Local::now();
    let timestamp = local_time.format("%H:%M:%S");

    for event in chatbox_events.iter() {
        if ui_state_chatbox.textbox_layout_job.sections.len() == MAX_CHATBOX_ENTRIES {
            ui_state_chatbox.textbox_layout_job.sections.remove(0);
            ui_state_chatbox.cleanup_layout_text_counter += 1;

            if ui_state_chatbox.cleanup_layout_text_counter == MAX_CHATBOX_ENTRIES {
                let offset = ui_state_chatbox.textbox_layout_job.sections[0]
                    .byte_range
                    .start;
                ui_state_chatbox.textbox_layout_job.text =
                    ui_state_chatbox.textbox_layout_job.text.split_off(offset);

                for section in ui_state_chatbox.textbox_layout_job.sections.iter_mut() {
                    section.byte_range.start -= offset;
                    section.byte_range.end -= offset;
                }

                ui_state_chatbox.cleanup_layout_text_counter = 0;
            }
        }

        ui_state_chatbox.textbox_layout_job.append(
            &format!("[{}] ", timestamp),
            0.0,
            egui::TextFormat {
                color: CHAT_COLOR_TIMESTAMP,
                ..Default::default()
            },
        );

        match event {
            ChatboxEvent::Say(name, text) => {
                ui_state_chatbox.textbox_layout_job.append(
                    &format!("{}> {}\n", name, text),
                    0.0,
                    egui::TextFormat {
                        color: CHAT_COLOR_NORMAL,
                        ..Default::default()
                    },
                );
            }
            ChatboxEvent::Shout(name, text) => {
                ui_state_chatbox.textbox_layout_job.append(
                    &format!("{}> {}\n", name, text),
                    0.0,
                    egui::TextFormat {
                        color: CHAT_COLOR_SHOUT,
                        ..Default::default()
                    },
                );
            }
            ChatboxEvent::Whisper(name, text) => {
                ui_state_chatbox.textbox_layout_job.append(
                    &format!("{}> {}\n", name, text),
                    0.0,
                    egui::TextFormat {
                        color: CHAT_COLOR_WHISPER,
                        ..Default::default()
                    },
                );
            }
            ChatboxEvent::Announce(Some(name), text) => {
                ui_state_chatbox.textbox_layout_job.append(
                    &format!("{}> {}\n", name, text),
                    0.0,
                    egui::TextFormat {
                        color: CHAT_COLOR_ANNOUNCE,
                        ..Default::default()
                    },
                );
            }
            ChatboxEvent::Announce(None, text) => {
                ui_state_chatbox.textbox_layout_job.append(
                    &format!("{}\n", text),
                    0.0,
                    egui::TextFormat {
                        color: CHAT_COLOR_ANNOUNCE,
                        ..Default::default()
                    },
                );
            }
            ChatboxEvent::System(text) => {
                ui_state_chatbox.textbox_layout_job.append(
                    &format!("{}\n", text),
                    0.0,
                    egui::TextFormat {
                        color: CHAT_COLOR_SYSTEM,
                        ..Default::default()
                    },
                );
            }
            ChatboxEvent::Quest(text) => {
                ui_state_chatbox.textbox_layout_job.append(
                    &format!("{}\n", text),
                    0.0,
                    egui::TextFormat {
                        color: CHAT_COLOR_QUEST,
                        ..Default::default()
                    },
                );
            }
        }
    }

    let mut chatbox_style = (*egui_context.ctx_mut().style()).clone();
    chatbox_style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgba_unmultiplied(
        chatbox_style.visuals.widgets.noninteractive.bg_fill.r(),
        chatbox_style.visuals.widgets.noninteractive.bg_fill.g(),
        chatbox_style.visuals.widgets.noninteractive.bg_fill.b(),
        128,
    );

    let style = egui_context.ctx_mut().style();
    let frame_fill = style.visuals.window_fill();
    let frame_fill =
        egui::Color32::from_rgba_unmultiplied(frame_fill.r(), frame_fill.g(), frame_fill.b(), 128);

    let mut response_editbox = None;
    let mut response_all_button = None;
    let mut response_whisper_button = None;
    let mut response_trade_button = None;
    let mut response_party_button = None;
    let mut response_clan_button = None;
    let mut response_allied_button = None;

    egui::Window::new("Chat Box")
        .anchor(egui::Align2::LEFT_BOTTOM, [0.0, 0.0])
        .frame(egui::Frame::none().fill(frame_fill))
        .title_bar(false)
        .resizable(false)
        .default_width(dialog.width)
        .default_height(dialog.height)
        .show(egui_context.ctx_mut(), |ui| {
            ui.visuals_mut().override_text_color =
                match ui_state_chatbox.textbox_text.chars().next() {
                    Some('!') => Some(CHAT_COLOR_SHOUT),
                    Some('@') => Some(CHAT_COLOR_WHISPER),
                    Some('#') => Some(CHAT_COLOR_PARTY),
                    Some('&') => Some(CHAT_COLOR_CLAN),
                    Some('~') => Some(CHAT_COLOR_ALLIED),
                    _ => Some(CHAT_COLOR_NORMAL),
                };

            dialog.draw(
                ui,
                DataBindings {
                    sound_events: Some(&mut ui_sound_events),
                    text: &mut [(IID_EDITBOX, &mut ui_state_chatbox.textbox_text)],
                    radio: &mut [(IID_RADIOBOX, &mut ui_state_chatbox.selected_channel)],
                    response: &mut [
                        (IID_EDITBOX, &mut response_editbox),
                        (IID_BTN_ALL, &mut response_all_button),
                        (IID_BTN_WHISPER, &mut response_whisper_button),
                        (IID_BTN_TRADE, &mut response_trade_button),
                        (IID_BTN_PARTY, &mut response_party_button),
                        (IID_BTN_CLAN, &mut response_clan_button),
                        (IID_BTN_ALLIED, &mut response_allied_button),
                    ],
                    visible: &mut [
                        (IID_CHAT_LIST_IMAGE, false),
                        (IID_LISTBOX_ALL, false),
                        (IID_SCROLLBAR_ALL, false),
                        (IID_LISTBOX_WHISPER, false),
                        (IID_SCROLLBAR_WHISPER, false),
                        (IID_LISTBOX_TRADE, false),
                        (IID_SCROLLBAR_TRADE, false),
                        (IID_LISTBOX_PARTY, false),
                        (IID_SCROLLBAR_PARTY, false),
                        (IID_LISTBOX_CLAN, false),
                        (IID_SCROLLBAR_CLAN, false),
                        (IID_LISTBOX_ALLIED, false),
                        (IID_SCROLLBAR_ALLIED, false),
                    ],
                    ..Default::default()
                },
                |ui, _bindings| {
                    ui.allocate_ui_at_rect(
                        egui::Rect::from_min_size(
                            ui.min_rect().min + egui::vec2(1.0, 0.0),
                            egui::vec2(390.0, 179.0),
                        ),
                        |ui| {
                            egui::ScrollArea::vertical()
                                .auto_shrink([false; 2])
                                .stick_to_bottom(true)
                                .show(ui, |ui| {
                                    ui.label(ui_state_chatbox.textbox_layout_job.clone());
                                });
                        },
                    );
                },
            );
        });

    if let Some(response) = response_editbox {
        if response
            .ctx
            .input(|input| input.key_pressed(egui::Key::Enter))
        {
            if response.lost_focus() {
                if !ui_state_chatbox.textbox_text.is_empty() {
                    // TODO: Parse text line to decide whether its chat, shout, etc
                    if let Some(game_connection) = game_connection.as_ref() {
                        game_connection
                            .client_message_tx
                            .send(ClientMessage::Chat {
                                text: ui_state_chatbox.textbox_text.clone(),
                            })
                            .ok();
                        ui_state_chatbox.textbox_text.clear();
                    }
                }
            } else {
                response.request_focus();
            }
        }
    }

    // TODO: Update filters when changing category
    if response_all_button.map_or(false, |r| r.clicked()) {
        ui_state_chatbox.textbox_text.clear();
    }

    if response_whisper_button.map_or(false, |r| r.clicked()) {
        ui_state_chatbox.textbox_text.clear();
        ui_state_chatbox.textbox_text.push('@');
    }

    if response_trade_button.map_or(false, |r| r.clicked()) {
        ui_state_chatbox.textbox_text.clear();
    }

    if response_party_button.map_or(false, |r| r.clicked()) {
        ui_state_chatbox.textbox_text.clear();
        ui_state_chatbox.textbox_text.push('#');
    }

    if response_clan_button.map_or(false, |r| r.clicked()) {
        ui_state_chatbox.textbox_text.clear();
        ui_state_chatbox.textbox_text.push('&');
    }

    if response_allied_button.map_or(false, |r| r.clicked()) {
        ui_state_chatbox.textbox_text.clear();
        ui_state_chatbox.textbox_text.push('~');
    }
}
