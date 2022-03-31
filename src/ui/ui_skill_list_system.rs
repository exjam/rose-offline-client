use bevy::prelude::{Local, Query, Res, ResMut, With};
use bevy_egui::{egui, EguiContext};

use rose_data::SkillPageType;
use rose_game_common::components::{SkillList, SkillSlot, SKILL_PAGE_SIZE};

use crate::{
    components::PlayerCharacter,
    resources::{GameData, Icons},
    ui::{DragAndDropId, DragAndDropSlot, UiStateDragAndDrop},
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
        |_| false,
        &mut ui_state_dnd.dragged_item,
        &mut dropped_item,
        [40.0, 40.0],
    ));

    if response.double_clicked() {
        // TODO: Use Skill
    }

    if let (Some(skill), Some(skill_data)) = (skill, skill_data) {
        response.on_hover_text(format!("{}\nSkill ID: {}", skill_data.name, skill.get(),));
    }
}

pub fn ui_skill_list_system(
    mut egui_context: ResMut<EguiContext>,
    mut ui_state_skill_list: Local<UiStateSkillList>,
    mut ui_state_dnd: ResMut<UiStateDragAndDrop>,
    query_player: Query<&SkillList, With<PlayerCharacter>>,
    game_data: Res<GameData>,
    icons: Res<Icons>,
) {
    let player_skill_list = query_player.single();

    egui::Window::new("Skill List")
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

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .always_show_scroll(true)
                .show(ui, |ui| {
                    egui::Grid::new("my_grid")
                        .num_columns(2)
                        .spacing([4.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            for i in 0..SKILL_PAGE_SIZE {
                                let skill_slot = SkillSlot(ui_state_skill_list.current_page, i);
                                ui_add_skill_list_slot(
                                    ui,
                                    skill_slot,
                                    player_skill_list,
                                    &game_data,
                                    &icons,
                                    &mut ui_state_dnd,
                                );

                                let skill = player_skill_list.get_skill(skill_slot);
                                let skill_data = skill
                                    .as_ref()
                                    .and_then(|skill| game_data.skills.get_skill(*skill));

                                if let Some(skill_data) = skill_data {
                                    ui.label(&skill_data.name);
                                }
                                ui.end_row();
                            }
                        });
                });
        });
}