use bevy::prelude::{Commands, Entity, Local, Query, Res, ResMut, State, With};
use bevy_egui::{egui, EguiContext};
use regex::Regex;

use rose_data::SkillId;
use rose_game_common::messages::client::ClientMessage;

use crate::{
    components::{Command, CommandCastSkillTarget, NextCommand, PlayerCharacter},
    resources::{
        AppState, GameConnection, GameData, SelectedTarget, UiResources, UiSpriteSheetType,
    },
    ui::{
        tooltips::{PlayerTooltipQuery, SkillTooltipType},
        ui_add_skill_tooltip, UiStateDebugWindows,
    },
};

#[derive(Default)]
pub struct UiStateDebugSkillList {
    filter_name: String,
    filter_castable: bool,
    filtered_skills: Vec<SkillId>,
}

pub fn ui_debug_skill_list_system(
    mut commands: Commands,
    mut egui_context: ResMut<EguiContext>,
    mut ui_state_debug_skill_list: Local<UiStateDebugSkillList>,
    mut ui_state_debug_windows: ResMut<UiStateDebugWindows>,
    app_state: Res<State<AppState>>,
    game_connection: Option<Res<GameConnection>>,
    game_data: Res<GameData>,
    ui_resources: Res<UiResources>,
    mut query_player: Query<(Entity, &mut Command), With<PlayerCharacter>>,
    query_player_tooltip: Query<PlayerTooltipQuery, With<PlayerCharacter>>,
    selected_target: Res<SelectedTarget>,
) {
    if !ui_state_debug_windows.debug_ui_open {
        return;
    }
    let player_tooltip_data = query_player_tooltip.get_single().ok();

    egui::Window::new("Skill List")
        .resizable(true)
        .default_height(300.0)
        .open(&mut ui_state_debug_windows.skill_list_open)
        .show(egui_context.ctx_mut(), |ui| {
            let mut filter_changed = false;

            egui::Grid::new("skill_list_controls_grid")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Skill Name Filter:");
                    if ui
                        .text_edit_singleline(&mut ui_state_debug_skill_list.filter_name)
                        .changed()
                    {
                        filter_changed = true;
                    }
                    ui.end_row();

                    ui.label("Only Castable:");
                    if ui
                        .checkbox(&mut ui_state_debug_skill_list.filter_castable, "Castable")
                        .changed()
                    {
                        filter_changed = true;
                    }
                    ui.end_row();
                });

            if ui_state_debug_skill_list.filter_name.is_empty()
                && ui_state_debug_skill_list.filtered_skills.is_empty()
            {
                filter_changed = true;
            }

            if filter_changed {
                let filter_name_re = if !ui_state_debug_skill_list.filter_name.is_empty() {
                    Some(
                        Regex::new(&format!(
                            "(?i){}",
                            regex::escape(&ui_state_debug_skill_list.filter_name)
                        ))
                        .unwrap(),
                    )
                } else {
                    None
                };

                ui_state_debug_skill_list.filtered_skills = game_data
                    .skills
                    .iter()
                    .filter_map(|skill_data| {
                        if (ui_state_debug_skill_list.filter_castable
                            && skill_data.casting_motion_id.is_none())
                            || !filter_name_re
                                .as_ref()
                                .map_or(true, |re| re.is_match(skill_data.name))
                        {
                            None
                        } else {
                            Some(skill_data.id)
                        }
                    })
                    .collect();
            }

            egui_extras::TableBuilder::new(ui)
                .striped(true)
                .cell_layout(egui::Layout::left_to_right().with_cross_align(egui::Align::Center))
                .column(egui_extras::Size::exact(45.0))
                .column(egui_extras::Size::initial(50.0).at_least(50.0))
                .column(egui_extras::Size::remainder().at_least(80.0))
                .column(egui_extras::Size::initial(100.0).at_least(100.0))
                .column(egui_extras::Size::initial(60.0).at_least(60.0))
                .column(egui_extras::Size::initial(60.0).at_least(60.0))
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.heading("Icon");
                    });
                    header.col(|ui| {
                        ui.heading("ID");
                    });
                    header.col(|ui| {
                        ui.heading("Name");
                    });
                    header.col(|ui| {
                        ui.heading("Type");
                    });
                    header.col(|ui| {
                        ui.heading("Action");
                    });
                    header.col(|ui| {
                        ui.heading("Action");
                    });
                })
                .body(|body| {
                    body.rows(
                        45.0,
                        ui_state_debug_skill_list.filtered_skills.len(),
                        |row_index, mut row| {
                            if let Some(skill_data) = ui_state_debug_skill_list
                                .filtered_skills
                                .get(row_index)
                                .and_then(|id| game_data.skills.get_skill(*id))
                            {
                                row.col(|ui| {
                                    if let Some(sprite) = ui_resources.get_sprite_by_index(
                                        UiSpriteSheetType::Skill,
                                        skill_data.icon_number as usize,
                                    ) {
                                        ui.add(
                                            egui::Image::new(sprite.texture_id, [40.0, 40.0])
                                                .uv(sprite.uv),
                                        )
                                        .on_hover_ui(
                                            |ui| {
                                                ui_add_skill_tooltip(
                                                    ui,
                                                    SkillTooltipType::Extra,
                                                    &game_data,
                                                    player_tooltip_data.as_ref(),
                                                    skill_data.id,
                                                );
                                            },
                                        );
                                    }
                                });

                                row.col(|ui| {
                                    ui.label(format!("{}", skill_data.id.get()));
                                });

                                row.col(|ui| {
                                    ui.label(skill_data.name);
                                });

                                row.col(|ui| {
                                    ui.label(format!("{:?}", skill_data.skill_type));
                                });

                                row.col(|ui| {
                                    if matches!(app_state.current(), AppState::Game)
                                        && ui.button("Learn").clicked()
                                    {
                                        if let Some(game_connection) = game_connection.as_ref() {
                                            game_connection
                                                .client_message_tx
                                                .send(ClientMessage::Chat(format!(
                                                    "/skill add {}",
                                                    skill_data.id.get()
                                                )))
                                                .ok();
                                        }
                                    }
                                });

                                row.col(|ui| {
                                    if let Ok((player_entity, mut player_command)) =
                                        query_player.get_single_mut()
                                    {
                                        if skill_data.casting_motion_id.is_some()
                                            && ui.button("Cast").clicked()
                                        {
                                            if let Command::CastSkill(command_cast_skill) =
                                                player_command.as_mut()
                                            {
                                                if command_cast_skill.skill_id == skill_data.id {
                                                    command_cast_skill.ready_action = true;
                                                } else {
                                                    *player_command = Command::with_stop();
                                                }
                                            } else {
                                                commands.entity(player_entity).insert(
                                                    NextCommand::with_cast_skill(
                                                        skill_data.id,
                                                        selected_target.selected.map(
                                                            |target_entity| {
                                                                CommandCastSkillTarget::Entity(
                                                                    target_entity,
                                                                )
                                                            },
                                                        ),
                                                        None,
                                                        None,
                                                        None,
                                                    ),
                                                );
                                            }
                                        }
                                    }
                                });
                            }
                        },
                    );
                });
        });
}
