use bevy::{
    ecs::query::WorldQuery,
    prelude::{Assets, Local, Query, Res, ResMut, With},
};
use bevy_egui::{egui, EguiContext};

use rose_data::SkillId;
use rose_game_common::components::{CharacterInfo, SkillList, SkillPoints};

use crate::{
    components::PlayerCharacter,
    resources::{GameData, UiResources, UiSpriteSheetType},
    ui::{
        tooltips::{PlayerTooltipQuery, PlayerTooltipQueryItem, SkillTooltipType},
        ui_add_skill_tooltip,
        widgets::{DataBindings, Dialog, DrawWidget, Skill, Widget},
        DragAndDropId, DragAndDropSlot, UiStateWindows,
    },
};

// const IID_BTN_ICONIZE: i32 = 10;
const IID_BTN_CLOSE: i32 = 11;

const IID_TEXT_SOLDIER: i32 = 21;
const IID_TEXT_MUSE: i32 = 22;
const IID_TEXT_HOWKER: i32 = 23;
const IID_TEXT_DEALER: i32 = 24;

#[derive(Default)]
pub struct UiStateSkillTree {
    skill_tree: Option<(u16, Dialog)>, // (job id, dialog)
}

fn ui_add_skill_tree_slot(
    ui: &mut egui::Ui,
    pos: egui::Pos2,
    skill: &Skill,
    player: &PlayerQueryItem,
    player_tooltip_data: Option<&PlayerTooltipQueryItem>,
    game_data: &GameData,
    ui_resources: &UiResources,
) -> egui::Response {
    let base_skill_id = if let Some(base_skill_id) = SkillId::new(skill.id as u16) {
        base_skill_id
    } else {
        return ui
            .allocate_ui_at_rect(
                egui::Rect::from_min_size(
                    ui.min_rect().min + pos.to_vec2(),
                    egui::vec2(40.0, 40.0),
                ),
                |_| {},
            )
            .response;
    };

    let learned_level = if let Some((_, _, level)) = player
        .skill_list
        .find_skill_level(&game_data.skills, base_skill_id)
    {
        if level < skill.level {
            None
        } else if skill.limit_level > 0 {
            Some(level.min(skill.limit_level))
        } else {
            Some(level)
        }
    } else {
        None
    };

    let skill_data = game_data.skills.get_skill(
        SkillId::new((skill.id + learned_level.unwrap_or(skill.level).max(1) - 1) as u16).unwrap(),
    );

    let sprite = skill_data.and_then(|skill_data| {
        ui_resources.get_sprite_by_index(UiSpriteSheetType::Skill, skill_data.icon_number as usize)
    });
    let mut dragged_item = None;
    let mut dropped_item = None;
    let response = ui
        .allocate_ui_at_rect(
            egui::Rect::from_min_size(ui.min_rect().min + pos.to_vec2(), egui::vec2(40.0, 40.0)),
            |ui| {
                egui::Widget::ui(
                    DragAndDropSlot::new(
                        DragAndDropId::NotDraggable,
                        sprite,
                        None,
                        if learned_level.is_some() {
                            None
                        } else {
                            Some(1.0)
                        },
                        |_| false,
                        &mut dragged_item,
                        &mut dropped_item,
                        [40.0, 40.0],
                    ),
                    ui,
                )
            },
        )
        .inner;

    if response.double_clicked() {
        // player_command_events.send(PlayerCommandEvent::UseSkill(skill_slot));
    }

    if let Some(skill_data) = skill_data {
        response.on_hover_ui(|ui| {
            ui_add_skill_tooltip(
                ui,
                SkillTooltipType::Extra,
                game_data,
                player_tooltip_data,
                skill_data.id,
            );
        })
    } else {
        response
    }
}

fn draw_skill_slots(
    ui: &mut egui::Ui,
    player: &PlayerQueryItem,
    player_tooltip_data: Option<&PlayerTooltipQueryItem>,
    game_data: &GameData,
    ui_resources: &UiResources,
    widgets: &[Widget],
) {
    for skill in widgets.iter().filter_map(|x| match x {
        Widget::Skill(s) => Some(s),
        _ => None,
    }) {
        ui_add_skill_tree_slot(
            ui,
            egui::pos2(skill.x + 3.0, skill.y + 3.0),
            skill,
            player,
            player_tooltip_data,
            game_data,
            ui_resources,
        );

        draw_skill_slots(
            ui,
            player,
            player_tooltip_data,
            game_data,
            ui_resources,
            &skill.widgets,
        );
    }
}

#[derive(WorldQuery)]
pub struct PlayerQuery<'w> {
    character_info: &'w CharacterInfo,
    skill_list: &'w SkillList,
    skill_points: &'w SkillPoints,
}

pub fn ui_skill_tree_system(
    mut egui_context: ResMut<EguiContext>,
    mut ui_state: Local<UiStateSkillTree>,
    mut ui_state_windows: ResMut<UiStateWindows>,
    query_player: Query<PlayerQuery, With<PlayerCharacter>>,
    query_player_tooltip: Query<PlayerTooltipQuery, With<PlayerCharacter>>,
    game_data: Res<GameData>,
    ui_resources: Res<UiResources>,
    dialog_assets: Res<Assets<Dialog>>,
) {
    let ui_state = &mut *ui_state;
    let dialog = if let Some(dialog) = dialog_assets.get(&ui_resources.dialog_skill_tree) {
        dialog
    } else {
        return;
    };

    let player = if let Ok(player) = query_player.get_single() {
        player
    } else {
        return;
    };
    let player_tooltip_data = query_player_tooltip.get_single().ok();

    if !ui_state
        .skill_tree
        .as_ref()
        .map_or(false, |(job, _)| *job == player.character_info.job)
    {
        let skill_tree = match player.character_info.job / 100 {
            1 => &ui_resources.skill_tree_soldier,
            2 => &ui_resources.skill_tree_muse,
            3 => &ui_resources.skill_tree_hawker,
            4 => &ui_resources.skill_tree_dealer,
            _ => return,
        };
        let skill_tree = if let Some(skill_tree) = dialog_assets.get(skill_tree) {
            skill_tree
        } else {
            return;
        };
        ui_state.skill_tree = Some((player.character_info.job, skill_tree.clone()));
    }
    let skill_tree = if let Some((_, skill_tree)) = ui_state.skill_tree.as_mut() {
        skill_tree
    } else {
        return;
    };

    let mut response_close_button = None;
    let mut select_base_skill_index = None;

    egui::Window::new("Skill Tree")
        .frame(egui::Frame::none())
        .open(&mut ui_state_windows.skill_tree_open)
        .title_bar(false)
        .resizable(false)
        .default_size([dialog.width, dialog.height])
        .show(egui_context.ctx_mut(), |ui| {
            dialog.draw(
                ui,
                DataBindings {
                    visible: &mut [
                        (IID_TEXT_SOLDIER, (player.character_info.job / 100) == 1),
                        (IID_TEXT_MUSE, (player.character_info.job / 100) == 2),
                        (IID_TEXT_HOWKER, (player.character_info.job / 100) == 3),
                        (IID_TEXT_DEALER, (player.character_info.job / 100) == 4),
                    ],
                    response: &mut [(IID_BTN_CLOSE, &mut response_close_button)],
                    ..Default::default()
                },
                |ui, bindings| {
                    // Draw all base skill icons
                    for (index, widget) in skill_tree.widgets.iter().enumerate() {
                        if let Widget::Skill(base_skill) = widget {
                            if ui_add_skill_tree_slot(
                                ui,
                                egui::pos2(base_skill.x + 3.0, base_skill.y + 3.0),
                                base_skill,
                                &player,
                                player_tooltip_data.as_ref(),
                                &game_data,
                                &ui_resources,
                            )
                            .clicked()
                                && index != 0
                            {
                                select_base_skill_index = Some(index);
                            }
                        }
                    }

                    // Draw only background & children of selected base skill
                    if let Some(Widget::Skill(base_skill)) = skill_tree.widgets.get(0) {
                        base_skill.draw_widget(ui, bindings);
                        draw_skill_slots(
                            ui,
                            &player,
                            player_tooltip_data.as_ref(),
                            &game_data,
                            &ui_resources,
                            &base_skill.widgets,
                        );
                    }
                },
            );
        });

    if let Some(new_skill_index) = select_base_skill_index {
        if let Widget::Skill(old_skill) = &skill_tree.widgets[0] {
            if let Widget::Skill(new_skill) = &skill_tree.widgets[new_skill_index] {
                let old_skill_pos = egui::pos2(old_skill.x, old_skill.y);
                let new_skill_pos = egui::pos2(new_skill.x, new_skill.y);
                skill_tree.widgets.swap(0, new_skill_index);

                if let Widget::Skill(new_skill) = &mut skill_tree.widgets[0] {
                    new_skill.x = old_skill_pos.x;
                    new_skill.y = old_skill_pos.y;
                }

                if let Widget::Skill(old_skill) = &mut skill_tree.widgets[new_skill_index] {
                    old_skill.x = new_skill_pos.x;
                    old_skill.y = new_skill_pos.y;
                }
            }
        }
    }

    if response_close_button.map_or(false, |r| r.clicked()) {
        ui_state_windows.skill_tree_open = false;
    }
}
