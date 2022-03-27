pub enum ChatboxEvent {
    Say(String, String),
    Shout(String, String),
    Whisper(String, String),
    Announce(Option<String>, String),
    System(String),
}

use bevy::prelude::{Commands, Entity, EventReader, Local, Query, Res, ResMut, With};
use bevy_egui::{egui, EguiContext};
use enum_map::{enum_map, EnumMap};
use rose_game_common::{
    components::{
        AbilityValues, CharacterInfo, ExperiencePoints, HealthPoints, Inventory, InventoryPageType,
        ItemSlot, Level, ManaPoints, Npc, INVENTORY_PAGE_SIZE,
    },
    messages::client::ClientMessage,
};

use crate::{
    components::{PlayerCharacter, SelectedTarget},
    events::ChatboxEvent,
    resources::{GameConnection, GameData},
};

#[derive(Default)]
pub struct UiStateChatbox {
    textbox_text: String,
    textbox_history: Vec<(egui::Color32, String)>,
}

pub fn ui_chatbox_system(
    mut commands: Commands,
    mut egui_context: ResMut<EguiContext>,
    mut ui_state: Local<UiChatboxState>,
    game_connection: Option<Res<GameConnection>>,
    mut chatbox_events: EventReader<ChatboxEvent>,
    game_data: Res<GameData>,
    query_player: Query<
        (
            Entity,
            Option<&SelectedTarget>,
            &AbilityValues,
            &CharacterInfo,
            &Inventory,
            &Level,
            &HealthPoints,
            &ManaPoints,
            &ExperiencePoints,
        ),
        With<PlayerCharacter>,
    >,
    query_target: Query<(
        &AbilityValues,
        &HealthPoints,
        Option<&Npc>,
        Option<&CharacterInfo>,
    )>,
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
                ui_state
                    .textbox_history
                    .push((egui::Color32::LIGHT_GRAY, format!("{}> {}", name, text)));
            }
            ChatboxEvent::Shout(name, text) => {
                ui_state
                    .textbox_history
                    .push((egui::Color32::LIGHT_BLUE, format!("{}> {}", name, text)));
            }
            ChatboxEvent::Whisper(name, text) => {
                ui_state
                    .textbox_history
                    .push((egui::Color32::LIGHT_GREEN, format!("{}> {}", name, text)));
            }
            ChatboxEvent::Announce(Some(name), text) => {
                ui_state
                    .textbox_history
                    .push((egui::Color32::LIGHT_RED, format!("{}> {}", name, text)));
            }
            ChatboxEvent::Announce(None, text) => {
                ui_state
                    .textbox_history
                    .push((egui::Color32::LIGHT_RED, text.clone()));
            }
            ChatboxEvent::System(text) => {
                ui_state
                    .textbox_history
                    .push((egui::Color32::from_rgb(255, 182, 193), text.clone()));
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
                egui::ScrollArea::vertical()
                    .max_height(250.0)
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        for (colour, text) in ui_state.textbox_history.iter() {
                            ui.colored_label(*colour, text);
                        }
                    });

                ui.text_edit_singleline(&mut ui_state.textbox_text);

                if ui.input().key_pressed(egui::Key::Enter) {
                    if let Some(game_connection) = game_connection.as_ref() {
                        game_connection
                            .client_message_tx
                            .send(ClientMessage::Chat(ui_state.textbox_text.clone()))
                            .ok();
                        ui_state.textbox_text.clear();
                    }
                }
            });
        });
}
