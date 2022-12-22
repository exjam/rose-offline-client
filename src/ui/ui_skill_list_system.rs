use bevy::{
    ecs::query::WorldQuery,
    prelude::{Assets, Entity, EventWriter, Local, Query, Res, ResMut, With},
};
use bevy_egui::{egui, EguiContext};

use rose_data::AbilityType;
use rose_data_irose::{IroseSkillPageType, SKILL_PAGE_SIZE};
use rose_game_common::components::{CharacterInfo, SkillList, SkillPoints, SkillSlot};

use crate::{
    bundles::ability_values_get_value,
    components::{Cooldowns, PlayerCharacter},
    events::PlayerCommandEvent,
    resources::{GameData, UiResources},
    ui::{
        tooltips::{PlayerTooltipQuery, PlayerTooltipQueryItem, SkillTooltipType},
        ui_add_skill_tooltip,
        widgets::{DataBindings, Dialog, DrawText, Widget},
        DragAndDropId, DragAndDropSlot, UiStateDragAndDrop, UiStateWindows,
    },
};

const IID_BTN_CLOSE: i32 = 10;
// const IID_BTN_ICONIZE: i32 = 11;
const IID_BTN_OPEN_SKILLTREE: i32 = 12;
const IID_TABBEDPANE: i32 = 20;

const IID_TAB_BASIC: i32 = 21;
// const IID_BTN_BASIC: i32 = 25;
const IID_ZLISTBOX_BASIC: i32 = 26;
// const IID_SCROLLBAR_BASIC: i32 = 27;

const IID_TAB_ACTIVE: i32 = 31;
// const IID_BTN_ACTIVE: i32 = 35;
const IID_ZLISTBOX_ACTIVE: i32 = 36;
// const IID_SCROLLBAR_ACTIVE: i32 = 37;

const IID_TAB_PASSIVE: i32 = 41;
// const IID_BTN_PASSIVE: i32 = 45;
const IID_ZLISTBOX_PASSIVE: i32 = 46;
// const IID_SCROLLBAR_PASSIVE: i32 = 47;

pub struct UiStateSkillList {
    current_page: i32,
    scroll_index_basic: i32,
    scroll_index_active: i32,
    scroll_index_passive: i32,
}

impl Default for UiStateSkillList {
    fn default() -> Self {
        Self {
            current_page: IID_TAB_BASIC,
            scroll_index_basic: 0,
            scroll_index_active: 0,
            scroll_index_passive: 0,
        }
    }
}

fn ui_add_skill_list_slot(
    ui: &mut egui::Ui,
    pos: egui::Pos2,
    skill_slot: SkillSlot,
    player: &PlayerQueryItem,
    player_tooltip_data: Option<&PlayerTooltipQueryItem>,
    game_data: &GameData,
    ui_resources: &UiResources,
    ui_state_dnd: &mut UiStateDragAndDrop,
    player_command_events: &mut EventWriter<PlayerCommandEvent>,
) {
    let skill = player.skill_list.get_skill(skill_slot);
    let mut dropped_item = None;
    let response = ui
        .allocate_ui_at_rect(
            egui::Rect::from_min_size(pos, egui::vec2(40.0, 40.0)),
            |ui| {
                egui::Widget::ui(
                    DragAndDropSlot::with_skill(
                        DragAndDropId::Skill(skill_slot),
                        skill.as_ref(),
                        Some(player.cooldowns),
                        game_data,
                        ui_resources,
                        |_| false,
                        &mut ui_state_dnd.dragged_item,
                        &mut dropped_item,
                        [40.0, 40.0],
                    ),
                    ui,
                )
            },
        )
        .inner;

    if response.double_clicked() {
        player_command_events.send(PlayerCommandEvent::UseSkill(skill_slot));
    }

    if let Some(skill_id) = skill {
        response.on_hover_ui(|ui| {
            let extra = ui.input().pointer.secondary_down();
            ui_add_skill_tooltip(
                ui,
                if extra {
                    SkillTooltipType::Extra
                } else {
                    SkillTooltipType::Detailed
                },
                game_data,
                player_tooltip_data,
                skill_id,
            );
        });
    }
}

#[derive(WorldQuery)]
pub struct PlayerQuery<'w> {
    character_info: &'w CharacterInfo,
    skill_list: &'w SkillList,
    skill_points: &'w SkillPoints,
    cooldowns: &'w Cooldowns,
}

pub fn ui_skill_list_system(
    mut egui_context: ResMut<EguiContext>,
    mut ui_state_skill_list: Local<UiStateSkillList>,
    mut ui_state_dnd: ResMut<UiStateDragAndDrop>,
    mut ui_state_windows: ResMut<UiStateWindows>,
    mut player_command_events: EventWriter<PlayerCommandEvent>,
    query_player: Query<PlayerQuery, With<PlayerCharacter>>,
    query_player_tooltip: Query<PlayerTooltipQuery, With<PlayerCharacter>>,
    game_data: Res<GameData>,
    ui_resources: Res<UiResources>,
    dialog_assets: Res<Assets<Dialog>>,
) {
    let ui_state_skill_list = &mut *ui_state_skill_list;
    let dialog = if let Some(dialog) = dialog_assets.get(&ui_resources.dialog_skill_list) {
        dialog
    } else {
        return;
    };

    let player = if let Ok(skill_list) = query_player.get_single() {
        skill_list
    } else {
        return;
    };
    let player_tooltip_data = query_player_tooltip.get_single().ok();

    let listbox_extent =
        if let Some(Widget::ZListbox(listbox)) = dialog.get_widget(IID_ZLISTBOX_BASIC) {
            listbox.extent
        } else {
            1
        };
    let scrollbar_range = 0..SKILL_PAGE_SIZE as i32;

    let mut response_close_button = None;
    let mut response_skill_tree_button = None;

    egui::Window::new("Skills")
        .frame(egui::Frame::none())
        .open(&mut ui_state_windows.skill_list_open)
        .title_bar(false)
        .resizable(false)
        .default_width(dialog.width)
        .default_height(dialog.height)
        .show(egui_context.ctx_mut(), |ui| {
            dialog.draw(
                ui,
                DataBindings {
                    tabs: &mut [(IID_TABBEDPANE, &mut ui_state_skill_list.current_page)],
                    scroll: &mut [
                        (
                            IID_ZLISTBOX_BASIC,
                            (
                                &mut ui_state_skill_list.scroll_index_basic,
                                scrollbar_range.clone(),
                                listbox_extent,
                            ),
                        ),
                        (
                            IID_ZLISTBOX_ACTIVE,
                            (
                                &mut ui_state_skill_list.scroll_index_active,
                                scrollbar_range.clone(),
                                listbox_extent,
                            ),
                        ),
                        (
                            IID_ZLISTBOX_PASSIVE,
                            (
                                &mut ui_state_skill_list.scroll_index_passive,
                                scrollbar_range.clone(),
                                listbox_extent,
                            ),
                        ),
                    ],
                    visible: &mut [(IID_BTN_OPEN_SKILLTREE, player.character_info.job != 0)],
                    label: &mut [(IID_BTN_OPEN_SKILLTREE, "Skill Tree")],
                    response: &mut [
                        (IID_BTN_CLOSE, &mut response_close_button),
                        (IID_BTN_OPEN_SKILLTREE, &mut response_skill_tree_button),
                    ],
                    ..Default::default()
                },
                |ui, bindings| {
                    let (page, index) = match bindings.get_tab(IID_TABBEDPANE) {
                        Some(&mut IID_TAB_BASIC) => (
                            IroseSkillPageType::Basic,
                            bindings.get_scroll(IID_ZLISTBOX_BASIC).map_or(0, |s| *s.0),
                        ),
                        Some(&mut IID_TAB_ACTIVE) => (
                            IroseSkillPageType::Active,
                            bindings.get_scroll(IID_ZLISTBOX_ACTIVE).map_or(0, |s| *s.0),
                        ),
                        Some(&mut IID_TAB_PASSIVE) => (
                            IroseSkillPageType::Passive,
                            bindings
                                .get_scroll(IID_ZLISTBOX_PASSIVE)
                                .map_or(0, |s| *s.0),
                        ),
                        _ => (IroseSkillPageType::Basic, 0),
                    };

                    let listbox_pos = egui::vec2(0.0, 65.0);
                    for i in 0..listbox_extent {
                        let skill_slot = SkillSlot(page as usize, (index + i) as usize);
                        let start_x = listbox_pos.x + 16.0;
                        let start_y = listbox_pos.y + 44.0 * i as f32;

                        let skill = player.skill_list.get_skill(skill_slot);
                        let skill_data = skill
                            .as_ref()
                            .and_then(|skill| game_data.skills.get_skill(*skill));
                        if let Some(skill_data) = skill_data {
                            // Skill name
                            if skill_data.level > 0 {
                                ui.add_label_at(
                                    egui::pos2(start_x + 46.0, start_y + 5.0),
                                    format!("{} (Lv: {})", skill_data.name, skill_data.level),
                                );
                            } else {
                                ui.add_label_at(
                                    egui::pos2(start_x + 46.0, start_y + 5.0),
                                    skill_data.name,
                                );
                            }

                            // Skill use ability values
                            if !skill_data.use_ability.is_empty() {
                                ui.allocate_ui_at_rect(
                                    egui::Rect::from_min_size(
                                        ui.min_rect().min
                                            + egui::vec2(start_x + 46.0, start_y + 25.0),
                                        egui::vec2(100.0, 18.0),
                                    ),
                                    |ui| {
                                        ui.horizontal(|ui| {
                                            for &(ability_type, mut value) in
                                                skill_data.use_ability.iter()
                                            {
                                                let mut color = egui::Color32::RED;

                                                if let Some(player_tooltip_data) =
                                                    player_tooltip_data.as_ref()
                                                {
                                                    if matches!(ability_type, AbilityType::Mana) {
                                                        let use_mana_rate = (100
                                                            - player_tooltip_data
                                                                .ability_values
                                                                .get_save_mana())
                                                            as f32
                                                            / 100.0;
                                                        value =
                                                            (value as f32 * use_mana_rate) as i32;
                                                    }

                                                    if let Some(current_value) =
                                                        ability_values_get_value(
                                                            ability_type,
                                                            player_tooltip_data.ability_values,
                                                            Some(
                                                                player_tooltip_data.character_info,
                                                            ),
                                                            Some(
                                                                player_tooltip_data
                                                                    .experience_points,
                                                            ),
                                                            Some(player_tooltip_data.health_points),
                                                            Some(player_tooltip_data.inventory),
                                                            Some(player_tooltip_data.level),
                                                            Some(player_tooltip_data.mana_points),
                                                            Some(player_tooltip_data.move_speed),
                                                            Some(player_tooltip_data.skill_points),
                                                            Some(player_tooltip_data.stamina),
                                                            Some(player_tooltip_data.stat_points),
                                                            Some(player_tooltip_data.team),
                                                            Some(
                                                                player_tooltip_data
                                                                    .union_membership,
                                                            ),
                                                        )
                                                    {
                                                        if current_value >= value {
                                                            color = egui::Color32::GREEN;
                                                        }
                                                    }
                                                }

                                                ui.colored_label(
                                                    color,
                                                    format!(
                                                        "{} {}",
                                                        game_data
                                                            .string_database
                                                            .get_ability_type(ability_type),
                                                        value
                                                    ),
                                                );
                                            }
                                        });
                                    },
                                );
                            }
                        }

                        // TODO: Skill level up button

                        ui_add_skill_list_slot(
                            ui,
                            ui.min_rect().min + egui::vec2(start_x, start_y + 3.0),
                            skill_slot,
                            &player,
                            player_tooltip_data.as_ref(),
                            &game_data,
                            &ui_resources,
                            &mut ui_state_dnd,
                            &mut player_command_events,
                        );
                    }

                    ui.add_label_at(
                        egui::pos2(40.0, dialog.height - 25.0),
                        &format!("{}", player.skill_points.points),
                    );
                },
            );
        });

    if response_skill_tree_button.map_or(false, |r| r.clicked()) {
        ui_state_windows.skill_tree_open = !ui_state_windows.skill_tree_open;
    }

    if response_close_button.map_or(false, |r| r.clicked()) {
        ui_state_windows.skill_list_open = false;
    }
}
