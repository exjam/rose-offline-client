use bevy::prelude::{Commands, Entity, EventReader, Local, Query, Res, ResMut, With};
use bevy_egui::{egui, EguiContext};
use rose_game_common::{
    components::{
        AbilityValues, CharacterInfo, ExperiencePoints, HealthPoints, Level, ManaPoints, Npc,
    },
    messages::client::ClientMessage,
};

use crate::{
    components::{PlayerCharacter, SelectedTarget},
    events::ChatboxEvent,
    resources::{GameConnection, GameData},
};

pub struct GameUiState {
    textbox_text: String,
    textbox_history: Vec<(egui::Color32, String)>,
}

impl Default for GameUiState {
    fn default() -> Self {
        Self {
            textbox_text: Default::default(),
            textbox_history: Default::default(),
        }
    }
}

pub fn game_ui_system(
    mut commands: Commands,
    mut egui_context: ResMut<EguiContext>,
    mut ui_state: Local<GameUiState>,
    game_connection: Option<Res<GameConnection>>,
    mut chatbox_events: EventReader<ChatboxEvent>,
    game_data: Res<GameData>,
    query_player: Query<
        (
            Entity,
            Option<&SelectedTarget>,
            &AbilityValues,
            &CharacterInfo,
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
    let (
        player_entity,
        player_target,
        player_ability_values,
        player_info,
        player_level,
        player_health_points,
        player_mana_points,
        player_experience_points,
    ) = query_player.single();

    egui::Window::new("Player Info")
        .anchor(egui::Align2::LEFT_TOP, [10.0, 30.0])
        .collapsible(false)
        .title_bar(false)
        .show(egui_context.ctx_mut(), |ui| {
            ui.label(&player_info.name);

            egui::Grid::new("player_info_grid")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("HP");
                    ui.scope(|ui| {
                        ui.style_mut().visuals.selection.bg_fill = egui::Color32::DARK_RED;
                        ui.add(
                            egui::ProgressBar::new(
                                player_health_points.hp as f32
                                    / player_ability_values.get_max_health() as f32,
                            )
                            .text(format!(
                                "{} / {}",
                                player_health_points.hp,
                                player_ability_values.get_max_health()
                            )),
                        )
                    });
                    ui.end_row();

                    ui.label("MP");
                    ui.scope(|ui| {
                        ui.style_mut().visuals.selection.bg_fill = egui::Color32::DARK_BLUE;
                        ui.add(
                            egui::ProgressBar::new(
                                player_mana_points.mp as f32
                                    / player_ability_values.get_max_mana() as f32,
                            )
                            .text(format!(
                                "{} / {}",
                                player_mana_points.mp,
                                player_ability_values.get_max_mana()
                            )),
                        );
                    });
                    ui.end_row();

                    ui.label("XP");
                    ui.scope(|ui| {
                        let need_xp = game_data
                            .ability_value_calculator
                            .calculate_levelup_require_xp(player_level.level);
                        ui.style_mut().visuals.selection.bg_fill =
                            egui::Color32::from_rgb(145, 133, 0);
                        ui.add(
                            egui::ProgressBar::new(
                                player_experience_points.xp as f32 / need_xp as f32,
                            )
                            .show_percentage(),
                        )
                        .on_hover_text(format!("{} / {}", player_experience_points.xp, need_xp));
                    });
                    ui.end_row();
                });

            ui.label(format!("Level {}", player_level.level));
        });

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
