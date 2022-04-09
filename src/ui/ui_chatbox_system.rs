use bevy::prelude::{EventReader, Local, Res, ResMut};
use bevy_egui::{egui, EguiContext};

use rose_game_common::messages::client::ClientMessage;

use crate::{events::ChatboxEvent, resources::GameConnection};

#[derive(Default)]
pub struct UiStateChatbox {
    textbox_text: String,
    textbox_history: Vec<(egui::Color32, String)>,
}

pub fn ui_chatbox_system(
    mut egui_context: ResMut<EguiContext>,
    mut ui_state_chatbox: Local<UiStateChatbox>,
    mut chatbox_events: EventReader<ChatboxEvent>,
    game_connection: Option<Res<GameConnection>>,
) {
    let mut chatbox_style = (*egui_context.ctx_mut().style()).clone();
    chatbox_style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgba_unmultiplied(
        chatbox_style.visuals.widgets.noninteractive.bg_fill.r(),
        chatbox_style.visuals.widgets.noninteractive.bg_fill.g(),
        chatbox_style.visuals.widgets.noninteractive.bg_fill.b(),
        128,
    );

    for event in chatbox_events.iter() {
        match event {
            ChatboxEvent::Say(name, text) => {
                ui_state_chatbox.textbox_history.push((
                    egui::Color32::from_rgb(255, 255, 255),
                    format!("{}> {}", name, text),
                ));
            }
            ChatboxEvent::Shout(name, text) => {
                ui_state_chatbox.textbox_history.push((
                    egui::Color32::from_rgb(189, 250, 255),
                    format!("{}> {}", name, text),
                ));
            }
            ChatboxEvent::Whisper(name, text) => {
                ui_state_chatbox.textbox_history.push((
                    egui::Color32::from_rgb(201, 255, 144),
                    format!("{}> {}", name, text),
                ));
            }
            ChatboxEvent::Announce(Some(name), text) => {
                ui_state_chatbox.textbox_history.push((
                    egui::Color32::from_rgb(255, 188, 172),
                    format!("{}> {}", name, text),
                ));
            }
            ChatboxEvent::Announce(None, text) => {
                ui_state_chatbox
                    .textbox_history
                    .push((egui::Color32::from_rgb(255, 188, 172), text.clone()));
            }
            ChatboxEvent::System(text) => {
                ui_state_chatbox
                    .textbox_history
                    .push((egui::Color32::from_rgb(255, 224, 229), text.clone()));
            }
            ChatboxEvent::Quest(text) => {
                ui_state_chatbox
                    .textbox_history
                    .push((egui::Color32::from_rgb(151, 221, 241), text.clone()));
            }
        }
    }

    egui::Window::new("Chat Box")
        .anchor(egui::Align2::LEFT_BOTTOM, [10.0, -10.0])
        .collapsible(false)
        .title_bar(false)
        .frame(egui::Frame::window(&chatbox_style))
        .show(egui_context.ctx_mut(), |ui| {
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                let text_style = egui::TextStyle::Body;
                let row_height = ui.text_style_height(&text_style);

                egui::ScrollArea::vertical()
                    .max_height(250.0)
                    .auto_shrink([false; 2])
                    .stick_to_bottom()
                    .show_rows(
                        ui,
                        row_height,
                        ui_state_chatbox.textbox_history.len(),
                        |ui, row_range| {
                            for row in row_range {
                                if let Some((colour, text)) =
                                    ui_state_chatbox.textbox_history.get(row)
                                {
                                    ui.colored_label(*colour, text);
                                }
                            }
                        },
                    );

                let response = ui.text_edit_singleline(&mut ui_state_chatbox.textbox_text);

                if ui.input().key_pressed(egui::Key::Enter) {
                    if response.lost_focus() {
                        if !ui_state_chatbox.textbox_text.is_empty() {
                            if let Some(game_connection) = game_connection.as_ref() {
                                game_connection
                                    .client_message_tx
                                    .send(ClientMessage::Chat(
                                        ui_state_chatbox.textbox_text.clone(),
                                    ))
                                    .ok();
                                ui_state_chatbox.textbox_text.clear();
                            }
                        }
                    } else {
                        response.request_focus();
                    }
                }
            });
        });
}
