use bevy::prelude::{EventWriter, Local, Query, Res, ResMut, With};
use bevy_egui::{egui, EguiContext};

use rose_data::SkillPageType;
use rose_game_common::components::{SkillList, SkillSlot, SKILL_PAGE_SIZE};

use crate::{
    components::PlayerCharacter,
    events::PlayerCommandEvent,
    resources::{GameData, Icons},
    ui::{
        ui_add_skill_tooltip, DragAndDropId, DragAndDropSlot, UiStateDragAndDrop, UiStateWindows,
    },
};

pub struct UiStateSkillList {
    current_page: SkillPageType,
}

impl Default for UiStateSkillList {
    fn default() -> Self {
        Self {
            current_page: SkillPageType::Basic,
        }
    }
}

fn ui_add_skill_list_slot(
    ui: &mut egui::Ui,
    skill_slot: SkillSlot,
    skill_list: &SkillList,
    game_data: &GameData,
    icons: &Icons,
    ui_state_dnd: &mut UiStateDragAndDrop,
    player_command_events: &mut EventWriter<PlayerCommandEvent>,
) {
    let skill = skill_list.get_skill(skill_slot);
    let skill_data = skill
        .as_ref()
        .and_then(|skill| game_data.skills.get_skill(*skill));
    let contents =
        skill_data.and_then(|skill_data| icons.get_skill_icon(skill_data.icon_number as usize));
    let mut dropped_item = None;
    let response = ui.add(DragAndDropSlot::new(
        DragAndDropId::Skill(skill_slot),
        contents,
        None,
        None,
        |_| false,
        &mut ui_state_dnd.dragged_item,
        &mut dropped_item,
        [40.0, 40.0],
    ));

    if response.double_clicked() {
        player_command_events.send(PlayerCommandEvent::UseSkill(skill_slot));
    }

    if let Some(skill_id) = skill {
        response.on_hover_ui(|ui| {
            ui_add_skill_tooltip(ui, false, game_data, skill_id);
        });
    }
}

pub fn ui_skill_list_system(
    mut egui_context: ResMut<EguiContext>,
    mut ui_state_skill_list: Local<UiStateSkillList>,
    mut ui_state_dnd: ResMut<UiStateDragAndDrop>,
    mut ui_state_windows: ResMut<UiStateWindows>,
    mut player_command_events: EventWriter<PlayerCommandEvent>,
    query_player: Query<&SkillList, With<PlayerCharacter>>,
    game_data: Res<GameData>,
    icons: Res<Icons>,
) {
    let player_skill_list = query_player.single();

    egui::Window::new("Skill List")
        .id(ui_state_windows.skill_list_window_id)
        .open(&mut ui_state_windows.skill_list_open)
        .resizable(true)
        .show(egui_context.ctx_mut(), |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut ui_state_skill_list.current_page,
                    SkillPageType::Basic,
                    "Basic",
                );
                ui.selectable_value(
                    &mut ui_state_skill_list.current_page,
                    SkillPageType::Active,
                    "Active",
                );
                ui.selectable_value(
                    &mut ui_state_skill_list.current_page,
                    SkillPageType::Passive,
                    "Passive",
                );
            });

            egui_extras::TableBuilder::new(ui)
                .striped(true)
                .cell_layout(egui::Layout::left_to_right().with_cross_align(egui::Align::Center))
                .column(egui_extras::Size::exact(45.0))
                .column(egui_extras::Size::remainder().at_least(80.0))
                .body(|body| {
                    body.rows(45.0, SKILL_PAGE_SIZE, |row_index, mut row| {
                        let skill_slot = SkillSlot(ui_state_skill_list.current_page, row_index);

                        row.col(|ui| {
                            ui_add_skill_list_slot(
                                ui,
                                skill_slot,
                                player_skill_list,
                                &game_data,
                                &icons,
                                &mut ui_state_dnd,
                                &mut player_command_events,
                            );
                        });

                        row.col(|ui| {
                            let skill = player_skill_list.get_skill(skill_slot);
                            let skill_data = skill
                                .as_ref()
                                .and_then(|skill| game_data.skills.get_skill(*skill));
                            if let Some(skill_data) = skill_data {
                                ui.label(&skill_data.name);
                            }
                        });
                    });
                });
        });
}
