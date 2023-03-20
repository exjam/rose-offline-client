use bevy::{
    ecs::query::WorldQuery,
    prelude::{AssetServer, Commands, Entity, Local, Query, Res, ResMut, State, With},
};
use bevy_egui::{egui, EguiContexts};
use regex::Regex;

use rose_data::{EquipmentIndex, SkillId};
use rose_game_common::{
    components::{CharacterGender, Equipment},
    messages::client::ClientMessage,
};

use crate::{
    components::{
        ActiveMotion, CharacterModel, Command, CommandCastSkill, CommandCastSkillState,
        CommandCastSkillTarget, NextCommand, PlayerCharacter,
    },
    resources::{
        AppState, GameConnection, GameData, SelectedTarget, UiResources, UiSpriteSheetType,
    },
    ui::{
        tooltips::{PlayerTooltipQuery, SkillTooltipType},
        ui_add_skill_tooltip, UiStateDebugWindows,
    },
};

#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct QueryCommand<'w> {
    entity: Entity,
    command: &'w mut Command,
}

#[derive(WorldQuery)]
pub struct QueryCharacter<'w> {
    entity: Entity,
    character_model: &'w CharacterModel,
    equipment: &'w Equipment,
}

#[derive(Default)]
pub struct UiStateDebugSkillList {
    filter_name: String,
    filter_castable: bool,
    filtered_skills: Vec<SkillId>,
}

pub fn ui_debug_skill_list_system(
    mut commands: Commands,
    mut egui_context: EguiContexts,
    mut ui_state_debug_skill_list: Local<UiStateDebugSkillList>,
    mut ui_state_debug_windows: ResMut<UiStateDebugWindows>,
    app_state: Res<State<AppState>>,
    asset_server: Res<AssetServer>,
    game_connection: Option<Res<GameConnection>>,
    game_data: Res<GameData>,
    ui_resources: Res<UiResources>,
    mut query_player_command: Query<QueryCommand, With<PlayerCharacter>>,
    query_character_models: Query<QueryCharacter, With<CharacterModel>>,
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
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(egui_extras::Column::exact(45.0))
                .column(egui_extras::Column::initial(50.0).at_least(50.0))
                .column(egui_extras::Column::remainder().at_least(80.0))
                .column(egui_extras::Column::initial(100.0).at_least(100.0))
                .column(egui_extras::Column::initial(100.0).at_least(100.0))
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
                                    if matches!(app_state.0, AppState::Game)
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

                                    if skill_data.casting_motion_id.is_some() {
                                        let player = query_player_command.get_single_mut().ok();

                                        if matches!(app_state.0, AppState::Game) {
                                            if let Some(mut player) = player {
                                                if let Command::CastSkill(command_cast_skill) =
                                                    player.command.as_mut()
                                                {
                                                    if command_cast_skill.skill_id
                                                        == skill_data.id && !command_cast_skill.ready_action
                                                    {
                                                        if ui.button("Action").clicked() {
                                                            command_cast_skill.ready_action = true;
                                                        }
                                                    } else if ui.button("Stop").clicked() {
                                                        *player.command = Command::with_stop();
                                                    }
                                                } else if ui.button("Cast").clicked() {
                                                    commands
                                                        .entity(player.entity)
                                                        .insert(NextCommand::with_cast_skill(
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
                                                    ));
                                                }
                                            }
                                        } else if matches!(
                                            app_state.0,
                                            AppState::ModelViewer
                                        ) {
                                            if ui.button("Cast").clicked() {
                                                for character in query_character_models.iter() {
                                                    let weapon_item_data = character
                                                        .equipment
                                                        .get_equipment_item(EquipmentIndex::Weapon)
                                                        .and_then(|weapon_item| {
                                                            game_data.items.get_weapon_item(
                                                                weapon_item.item.item_number,
                                                            )
                                                        });
                                                    let weapon_motion_type = weapon_item_data
                                                        .map(|weapon_item_data| {
                                                            weapon_item_data.motion_type as usize
                                                        })
                                                        .unwrap_or(0);
                                                    let weapon_motion_gender =
                                                        match character.character_model.gender {
                                                            CharacterGender::Male => 0,
                                                            CharacterGender::Female => 1,
                                                        };

                                                    let motion_data = skill_data
                                                        .casting_motion_id
                                                        .and_then(|motion_id| {
                                                            game_data
                                                                .character_motion_database
                                                                .find_first_character_motion(
                                                                    motion_id,
                                                                    weapon_motion_type,
                                                                    weapon_motion_gender,
                                                                )
                                                        });

                                                    if let Some(motion_data) = motion_data {
                                                        commands.entity(character.entity)
                                                        .insert(Command::CastSkill(CommandCastSkill {
                                                            skill_id: skill_data.id,
                                                            skill_target: None,
                                                            action_motion_id: skill_data.action_motion_id,
                                                            cast_motion_id: skill_data.casting_motion_id,
                                                            cast_repeat_motion_id: skill_data.casting_repeat_motion_id,
                                                            cast_skill_state: CommandCastSkillState::Casting,
                                                            ready_action: true,
                                                        }))
                                                        .insert(
                                                            ActiveMotion::new_once(
                                                                asset_server
                                                                    .load(motion_data.path.path()),
                                                            )
                                                            .with_animation_speed(
                                                                skill_data.casting_motion_speed,
                                                            ),
                                                        );
                                                    }
                                                }
                                            }

                                            if ui.button("Action").clicked() {
                                                for character in query_character_models.iter() {
                                                    let weapon_item_data = character
                                                        .equipment
                                                        .get_equipment_item(EquipmentIndex::Weapon)
                                                        .and_then(|weapon_item| {
                                                            game_data.items.get_weapon_item(
                                                                weapon_item.item.item_number,
                                                            )
                                                        });
                                                    let weapon_motion_type = weapon_item_data
                                                        .map(|weapon_item_data| {
                                                            weapon_item_data.motion_type as usize
                                                        })
                                                        .unwrap_or(0);
                                                    let weapon_motion_gender =
                                                        match character.character_model.gender {
                                                            CharacterGender::Male => 0,
                                                            CharacterGender::Female => 1,
                                                        };

                                                    let motion_data = skill_data
                                                        .action_motion_id
                                                        .and_then(|motion_id| {
                                                            game_data
                                                                .character_motion_database
                                                                .find_first_character_motion(
                                                                    motion_id,
                                                                    weapon_motion_type,
                                                                    weapon_motion_gender,
                                                                )
                                                        });

                                                    if let Some(motion_data) = motion_data {
                                                        commands.entity(character.entity)
                                                        .insert(Command::CastSkill(CommandCastSkill {
                                                            skill_id: skill_data.id,
                                                            skill_target: None,
                                                            action_motion_id: skill_data.action_motion_id,
                                                            cast_motion_id: skill_data.casting_motion_id,
                                                            cast_repeat_motion_id: skill_data.casting_repeat_motion_id,
                                                            cast_skill_state: CommandCastSkillState::Action,
                                                            ready_action: true,
                                                        })).insert(
                                                            ActiveMotion::new_once(
                                                                asset_server
                                                                    .load(motion_data.path.path()),
                                                            )
                                                            .with_animation_speed(
                                                                skill_data.action_motion_speed,
                                                            ),
                                                        );
                                                    }
                                                }
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
