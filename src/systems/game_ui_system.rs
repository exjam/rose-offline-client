use bevy::prelude::{Commands, Entity, EventReader, Local, Query, Res, ResMut, With};
use bevy_egui::{egui, EguiContext};
use rose_game_common::{
    components::{AbilityValues, CharacterInfo, HealthPoints, Npc},
    messages::client::ClientMessage,
};

use crate::{
    components::{PlayerCharacter, SelectedTarget},
    events::ChatboxEvent,
    resources::{GameConnection, GameData},
};

#[derive(Default)]
pub struct GameUiState {
    textbox_text: String,
    textbox_history: Vec<(egui::Color32, String)>,
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn game_ui_system(
    mut commands: Commands,
    mut egui_context: ResMut<EguiContext>,
    mut ui_state: Local<GameUiState>,
    game_connection: Option<Res<GameConnection>>,
    mut chatbox_events: EventReader<ChatboxEvent>,
    game_data: Res<GameData>,
    query_player: Query<(Entity, Option<&SelectedTarget>), With<PlayerCharacter>>,
    query_target: Query<(
        &AbilityValues,
        &HealthPoints,
        Option<&Npc>,
        Option<&CharacterInfo>,
    )>,
) {
    let (player_entity, player_target) = query_player.single();
    if let Some(player_target) = player_target {
        if let Ok((ability_values, health_points, npc, character_info)) =
            query_target.get(player_target.entity)
        {
            egui::Window::new("Player Target")
                .anchor(egui::Align2::CENTER_TOP, [0.0, 30.0])
                .collapsible(false)
                .title_bar(false)
                .show(egui_context.ctx_mut(), |ui| {
                    if let Some(npc_data) = npc.and_then(|npc| game_data.npcs.get_npc(npc.id)) {
                        ui.label(&npc_data.name);
                    } else if let Some(character_info) = character_info {
                        ui.label(&character_info.name);
                    } else {
                        ui.label("???");
                    }

                    ui.label(format!("Level: {}", ability_values.level));

                    ui.scope(|ui| {
                        ui.style_mut().visuals.selection.bg_fill = egui::Color32::DARK_RED;
                        ui.add(
                            egui::ProgressBar::new(
                                health_points.hp as f32 / ability_values.get_max_health() as f32,
                            )
                            .show_percentage(),
                        )
                        .on_hover_text(format!(
                            "{} / {}",
                            health_points.hp,
                            ability_values.get_max_health()
                        ));
                    });
                });
        } else {
            // Selected target no longer valid, remove it
            commands.entity(player_entity).remove::<SelectedTarget>();
        }
    }

    // Chat box
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
        }
    }

    egui::Window::new("Chat Box")
        .anchor(egui::Align2::LEFT_BOTTOM, [5.0, -5.0])
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
