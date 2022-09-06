use bevy::prelude::{Local, Query, Res, ResMut, State};
use bevy_egui::{egui, EguiContext};
use rand::Rng;
use regex::Regex;

use rose_data::NpcId;
use rose_game_common::{
    components::{Npc, Team},
    messages::client::ClientMessage,
};

use crate::{
    resources::{AppState, GameConnection, GameData},
    ui::UiStateDebugWindows,
};

#[derive(Debug, PartialEq, Eq)]
pub enum UiStateSpawnNpcTeam {
    Character,
    Monster,
    Random,
}

pub struct UiStateDebugNpcList {
    filter_name: String,
    filtered_npcs: Vec<NpcId>,
    spawn_count: usize,
    spawn_distance: usize,
    spawn_team: UiStateSpawnNpcTeam,
}

impl Default for UiStateDebugNpcList {
    fn default() -> Self {
        Self {
            filter_name: String::default(),
            filtered_npcs: Vec::default(),
            spawn_count: 1,
            spawn_distance: 250,
            spawn_team: UiStateSpawnNpcTeam::Monster,
        }
    }
}

pub fn ui_debug_npc_list_system(
    mut egui_context: ResMut<EguiContext>,
    mut ui_state_debug_npc_list: Local<UiStateDebugNpcList>,
    mut ui_state_debug_windows: ResMut<UiStateDebugWindows>,
    app_state: Res<State<AppState>>,
    game_connection: Option<Res<GameConnection>>,
    game_data: Res<GameData>,
    mut query_npc: Query<&mut Npc>,
) {
    if !ui_state_debug_windows.debug_ui_open {
        return;
    }

    egui::Window::new("NPC List")
        .resizable(true)
        .default_height(300.0)
        .open(&mut ui_state_debug_windows.npc_list_open)
        .show(egui_context.ctx_mut(), |ui| {
            let mut filter_changed = false;

            egui::Grid::new("npc_list_controls_grid")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("NPC Name Filter:");
                    if ui
                        .text_edit_singleline(&mut ui_state_debug_npc_list.filter_name)
                        .changed()
                    {
                        filter_changed = true;
                    }
                    ui.end_row();

                    if matches!(app_state.current(), AppState::Game) {
                        ui.label("Spawn Count:");
                        ui.add(
                            egui::DragValue::new(&mut ui_state_debug_npc_list.spawn_count)
                                .speed(1)
                                .clamp_range(1..=1000),
                        );
                        ui.end_row();

                        ui.label("Spawn Distance:");
                        ui.add(
                            egui::DragValue::new(&mut ui_state_debug_npc_list.spawn_distance)
                                .speed(1),
                        );
                        ui.end_row();

                        ui.label("Spawn Team:");
                        egui::ComboBox::from_id_source("npc_list_controls_team")
                            .selected_text(format!("{:?}", ui_state_debug_npc_list.spawn_team))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut ui_state_debug_npc_list.spawn_team,
                                    UiStateSpawnNpcTeam::Monster,
                                    "Monster",
                                );
                                ui.selectable_value(
                                    &mut ui_state_debug_npc_list.spawn_team,
                                    UiStateSpawnNpcTeam::Character,
                                    "Character",
                                );
                                ui.selectable_value(
                                    &mut ui_state_debug_npc_list.spawn_team,
                                    UiStateSpawnNpcTeam::Random,
                                    "Random",
                                );
                            });
                        ui.end_row();
                    }
                });

            if ui_state_debug_npc_list.filter_name.is_empty()
                && ui_state_debug_npc_list.filtered_npcs.is_empty()
            {
                filter_changed = true;
            }

            if filter_changed {
                if ui_state_debug_npc_list.filter_name.is_empty() {
                    ui_state_debug_npc_list.filtered_npcs =
                        game_data.npcs.iter().map(|npc_data| npc_data.id).collect();
                } else {
                    let re = Regex::new(&format!(
                        "(?i){}",
                        regex::escape(&ui_state_debug_npc_list.filter_name)
                    ))
                    .unwrap();

                    ui_state_debug_npc_list.filtered_npcs = game_data
                        .npcs
                        .iter()
                        .filter_map(|npc_data| {
                            if re.is_match(npc_data.name) {
                                Some(npc_data.id)
                            } else {
                                None
                            }
                        })
                        .collect();
                }
            }

            egui_extras::TableBuilder::new(ui)
                .striped(true)
                .cell_layout(egui::Layout::left_to_right().with_cross_align(egui::Align::Center))
                .column(egui_extras::Size::initial(50.0).at_least(50.0))
                .column(egui_extras::Size::remainder().at_least(80.0))
                .column(egui_extras::Size::initial(60.0).at_least(60.0))
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.heading("ID");
                    });
                    header.col(|ui| {
                        ui.heading("Name");
                    });
                    header.col(|ui| {
                        ui.heading("Action");
                    });
                })
                .body(|body| {
                    body.rows(
                        20.0,
                        ui_state_debug_npc_list.filtered_npcs.len(),
                        |row_index, mut row| {
                            if let Some(npc_data) = ui_state_debug_npc_list
                                .filtered_npcs
                                .get(row_index)
                                .and_then(|id| game_data.npcs.get_npc(*id))
                            {
                                row.col(|ui| {
                                    ui.label(format!("{}", npc_data.id.get()));
                                });

                                row.col(|ui| {
                                    ui.label(npc_data.name);
                                });

                                row.col(|ui| match app_state.current() {
                                    AppState::Game => {
                                        if ui
                                            .add_enabled(
                                                npc_data.npc_type_index.map_or(0, |x| x.get())
                                                    < 900,
                                                egui::Button::new("Spawn"),
                                            )
                                            .clicked()
                                        {
                                            if let Some(game_connection) = game_connection.as_ref()
                                            {
                                                let team_id = match ui_state_debug_npc_list
                                                    .spawn_team
                                                {
                                                    UiStateSpawnNpcTeam::Character => {
                                                        Team::DEFAULT_CHARACTER_TEAM_ID
                                                    }
                                                    UiStateSpawnNpcTeam::Monster => {
                                                        Team::DEFAULT_MONSTER_TEAM_ID
                                                    }
                                                    UiStateSpawnNpcTeam::Random => {
                                                        Team::UNIQUE_TEAM_ID_BASE
                                                            + rand::thread_rng().gen_range(0..9999)
                                                    }
                                                };

                                                game_connection
                                                    .client_message_tx
                                                    .send(ClientMessage::Chat(format!(
                                                        "/mon {} {} {} {}",
                                                        npc_data.id.get(),
                                                        ui_state_debug_npc_list.spawn_count,
                                                        ui_state_debug_npc_list.spawn_distance,
                                                        team_id,
                                                    )))
                                                    .ok();
                                            }
                                        }
                                    }
                                    AppState::ModelViewer => {
                                        if ui.button("View").clicked() {
                                            for mut npc in query_npc.iter_mut() {
                                                npc.id = npc_data.id;
                                            }
                                        }
                                    }
                                    _ => {}
                                });
                            }
                        },
                    );
                });
        });
}
