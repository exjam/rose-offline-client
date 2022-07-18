use bevy::prelude::{EventReader, Local, Res, ResMut};
use bevy_egui::{egui, EguiContext};

use rose_game_common::messages::client::ClientMessage;

use crate::{events::ChatboxEvent, resources::GameConnection};

const MAX_CHATBOX_ENTRIES: usize = 100;

#[derive(Default)]
pub struct UiStateChatbox {
    textbox_text: String,
    textbox_layout_job: egui::text::LayoutJob,
    cleanup_layout_text_counter: usize,
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
                color: egui::Color32::from_rgb(150, 150, 150),
                ..Default::default()
            },
        );

        match event {
            ChatboxEvent::Say(name, text) => {
                ui_state_chatbox.textbox_layout_job.append(
                    &format!("{}> {}\n", name, text),
                    0.0,
                    egui::TextFormat {
                        color: egui::Color32::from_rgb(255, 255, 255),
                        ..Default::default()
                    },
                );
            }
            ChatboxEvent::Shout(name, text) => {
                ui_state_chatbox.textbox_layout_job.append(
                    &format!("{}> {}\n", name, text),
                    0.0,
                    egui::TextFormat {
                        color: egui::Color32::from_rgb(189, 250, 255),
                        ..Default::default()
                    },
                );
            }
            ChatboxEvent::Whisper(name, text) => {
                ui_state_chatbox.textbox_layout_job.append(
                    &format!("{}> {}\n", name, text),
                    0.0,
                    egui::TextFormat {
                        color: egui::Color32::from_rgb(201, 255, 144),
                        ..Default::default()
                    },
                );
            }
            ChatboxEvent::Announce(Some(name), text) => {
                ui_state_chatbox.textbox_layout_job.append(
                    &format!("{}> {}\n", name, text),
                    0.0,
                    egui::TextFormat {
                        color: egui::Color32::from_rgb(255, 188, 172),
                        ..Default::default()
                    },
                );
            }
            ChatboxEvent::Announce(None, text) => {
                ui_state_chatbox.textbox_layout_job.append(
                    &format!("{}\n", text),
                    0.0,
                    egui::TextFormat {
                        color: egui::Color32::from_rgb(255, 188, 172),
                        ..Default::default()
                    },
                );
            }
            ChatboxEvent::System(text) => {
                ui_state_chatbox.textbox_layout_job.append(
                    &format!("{}\n", text),
                    0.0,
                    egui::TextFormat {
                        color: egui::Color32::from_rgb(255, 224, 229),
                        ..Default::default()
                    },
                );
            }
            ChatboxEvent::Quest(text) => {
                ui_state_chatbox.textbox_layout_job.append(
                    &format!("{}\n", text),
                    0.0,
                    egui::TextFormat {
                        color: egui::Color32::from_rgb(151, 221, 241),
                        ..Default::default()
                    },
                );
            }
        }
    }

    egui::Window::new("Chat Box")
        .anchor(egui::Align2::LEFT_BOTTOM, [10.0, -10.0])
        .collapsible(false)
        .title_bar(false)
        .default_size([350.0, 300.0])
        .frame(egui::Frame::window(&chatbox_style))
        .show(egui_context.ctx_mut(), |ui| {
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                egui_extras::StripBuilder::new(ui)
                    .size(egui_extras::Size::remainder().at_least(50.0))
                    .size(egui_extras::Size::exact(20.0))
                    .vertical(|mut strip| {
                        strip.cell(|ui| {
                            ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                                egui::ScrollArea::vertical()
                                    .auto_shrink([false; 2])
                                    .stick_to_bottom()
                                    .show(ui, |ui| {
                                        ui.label(ui_state_chatbox.textbox_layout_job.clone());
                                    });
                            });
                        });

                        strip.cell(|ui| {
                            let response =
                                ui.text_edit_singleline(&mut ui_state_chatbox.textbox_text);

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
            });
        });
}
