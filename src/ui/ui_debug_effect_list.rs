use bevy::prelude::{
    Commands, ComputedVisibility, DespawnRecursiveExt, Entity, EventWriter, GlobalTransform, Local,
    Query, Res, ResMut, Transform, Visibility, With,
};
use bevy_egui::{egui, EguiContexts};
use regex::Regex;

use rose_data::EffectFileId;

use crate::{
    components::{Effect, PlayerCharacter},
    events::{SpawnEffectData, SpawnEffectEvent},
    resources::{GameData, SelectedTarget},
    ui::UiStateDebugWindows,
};

#[derive(Default)]
pub struct UiStateDebugEffectList {
    filter_name: String,
    last_effect_entity: Option<Entity>,
    filtered_effects: Vec<EffectFileId>,
}

pub fn ui_debug_effect_list_system(
    mut commands: Commands,
    mut egui_context: EguiContexts,
    mut ui_state: Local<UiStateDebugEffectList>,
    mut ui_state_debug_windows: ResMut<UiStateDebugWindows>,
    game_data: Res<GameData>,
    mut spawn_effect_events: EventWriter<SpawnEffectEvent>,
    query_effects: Query<Entity, With<Effect>>,
    query_global_transform: Query<&GlobalTransform>,
    query_player: Query<Entity, With<PlayerCharacter>>,
    selected_target: Res<SelectedTarget>,
) {
    if !ui_state_debug_windows.debug_ui_open {
        return;
    }

    egui::Window::new("Effect List")
        .vscroll(true)
        .resizable(true)
        .default_height(300.0)
        .open(&mut ui_state_debug_windows.effect_list_open)
        .show(egui_context.ctx_mut(), |ui| {
            let mut filter_changed = false;

            egui::Grid::new("effect_list_controls_grid")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Effect Path Filter:");
                    if ui.text_edit_singleline(&mut ui_state.filter_name).changed() {
                        filter_changed = true;
                    }
                    ui.end_row();

                    ui.label("Despawn:");

                    let enabled = ui_state
                        .last_effect_entity
                        .map_or(false, |entity| query_effects.get(entity).is_ok());
                    ui.add_enabled_ui(enabled, |ui| {
                        if ui.button("Despawn").clicked() {
                            if let Some(last_effect_entity) = ui_state.last_effect_entity.take() {
                                if query_effects.get(last_effect_entity).is_ok() {
                                    commands.entity(last_effect_entity).despawn_recursive();
                                }
                            }
                        }
                    });
                    ui.end_row();
                });

            if ui_state.filter_name.is_empty() && ui_state.filtered_effects.is_empty() {
                filter_changed = true;
            }

            if filter_changed {
                let filter_name_re = if !ui_state.filter_name.is_empty() {
                    Some(
                        Regex::new(&format!("(?i){}", regex::escape(&ui_state.filter_name)))
                            .unwrap(),
                    )
                } else {
                    None
                };

                ui_state.filtered_effects = game_data
                    .effect_database
                    .iter_files()
                    .filter_map(|(effect_file_id, effect_file_path)| {
                        if !filter_name_re.as_ref().map_or(true, |re| {
                            re.is_match(effect_file_path.path().to_str().unwrap_or(""))
                        }) {
                            None
                        } else {
                            Some(effect_file_id)
                        }
                    })
                    .collect();
            }

            egui_extras::TableBuilder::new(ui)
                .striped(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(egui_extras::Column::initial(50.0).at_least(50.0))
                .column(egui_extras::Column::remainder().at_least(50.0))
                .column(egui_extras::Column::initial(60.0).at_least(60.0))
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.heading("ID");
                    });
                    header.col(|ui| {
                        ui.heading("Path");
                    });
                    header.col(|ui| {
                        ui.heading("Action");
                    });
                })
                .body(|body| {
                    body.rows(
                        20.0,
                        ui_state.filtered_effects.len(),
                        |row_index, mut row| {
                            let effect_file_id = ui_state.filtered_effects[row_index];
                            let effect_file_path = game_data
                                .effect_database
                                .get_effect_file(effect_file_id)
                                .unwrap();

                            row.col(|ui| {
                                ui.label(format!("{}", effect_file_id.get()));
                            });

                            row.col(|ui| {
                                ui.label(effect_file_path.path().to_string_lossy().as_ref());
                            });

                            row.col(|ui| {
                                if ui.button("View").clicked() {
                                    if let Some(last_effect_entity) =
                                        ui_state.last_effect_entity.take()
                                    {
                                        if query_effects.get(last_effect_entity).is_ok() {
                                            commands.entity(last_effect_entity).despawn_recursive();
                                        }
                                    }

                                    let transform = Transform::from(
                                        selected_target
                                            .selected
                                            .or_else(|| query_player.get_single().ok())
                                            .and_then(|target_entity| {
                                                query_global_transform.get(target_entity).ok()
                                            })
                                            .cloned()
                                            .unwrap_or_default(),
                                    );

                                    let effect_entity = commands
                                        .spawn((
                                            transform,
                                            GlobalTransform::default(),
                                            Visibility::default(),
                                            ComputedVisibility::default(),
                                        ))
                                        .id();

                                    spawn_effect_events.send(SpawnEffectEvent::InEntity(
                                        effect_entity,
                                        SpawnEffectData::with_path(effect_file_path.clone()),
                                    ));

                                    ui_state.last_effect_entity = Some(effect_entity);
                                }
                            });
                        },
                    );
                });
        });
}
