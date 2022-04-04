use bevy::prelude::{Local, Mut, Query, Res, ResMut, With};
use bevy_egui::{egui, EguiContext};

use rose_data::Item;
use rose_game_common::components::{
    Equipment, Hotbar, HotbarSlot, Inventory, SkillList, HOTBAR_NUM_PAGES, HOTBAR_PAGE_SIZE,
};

use crate::{
    components::PlayerCharacter,
    resources::{GameData, Icons},
    ui::{DragAndDropId, DragAndDropSlot, UiStateDragAndDrop},
};

use super::ui_inventory_system::GetItem;

#[derive(Default)]
pub struct UiStateHotBar {
    current_page: usize,
}

fn hotbar_drag_accepts(drag_source: &DragAndDropId) -> bool {
    matches!(
        drag_source,
        DragAndDropId::Inventory(_) | DragAndDropId::Skill(_) | DragAndDropId::Hotbar(_, _)
    )
}

fn ui_add_hotbar_slot(
    ui: &mut egui::Ui,
    hotbar_index: (usize, usize),
    player_hotbar: &mut Mut<Hotbar>,
    player_equipment: &Equipment,
    player_inventory: &Inventory,
    player_skill_list: &SkillList,
    game_data: &GameData,
    icons: &Icons,
    ui_state_dnd: &mut UiStateDragAndDrop,
) {
    let hotbar_slot = player_hotbar.pages[hotbar_index.0][hotbar_index.1].as_ref();
    let (contents, quantity) = match hotbar_slot {
        Some(HotbarSlot::Skill(skill_slot)) => {
            let skill = player_skill_list.get_skill(*skill_slot);
            let skill_data = skill
                .as_ref()
                .and_then(|skill| game_data.skills.get_skill(*skill));
            (
                skill_data
                    .and_then(|skill_data| icons.get_skill_icon(skill_data.icon_number as usize)),
                None,
            )
        }
        Some(HotbarSlot::Inventory(item_slot)) => {
            let item = (player_equipment, player_inventory).get_item(*item_slot);
            let item_data = item
                .as_ref()
                .and_then(|item| game_data.items.get_base_item(item.get_item_reference()));
            (
                item_data.and_then(|item_data| icons.get_item_icon(item_data.icon_index as usize)),
                match item {
                    Some(Item::Stackable(stackable_item)) => Some(stackable_item.quantity as usize),
                    _ => None,
                },
            )
        }
        _ => (None, None),
    };

    let mut dropped_item = None;
    let response = ui.add(DragAndDropSlot::new(
        DragAndDropId::Hotbar(hotbar_index.0, hotbar_index.1),
        contents,
        quantity,
        hotbar_drag_accepts,
        &mut ui_state_dnd.dragged_item,
        &mut dropped_item,
        [40.0, 40.0],
    ));

    if response.double_clicked() {
        // TODO: Use hot bar
    }

    // TODO: Send to server
    match dropped_item {
        Some(DragAndDropId::Hotbar(page, index)) => {
            if page != hotbar_index.0 || index != hotbar_index.1 {
                let slot_a = player_hotbar.pages[hotbar_index.0][hotbar_index.1].take();
                let slot_b = player_hotbar.pages[page][index].take();

                player_hotbar.pages[page][index] = slot_a;
                player_hotbar.pages[hotbar_index.0][hotbar_index.1] = slot_b;
            }
        }
        Some(DragAndDropId::Inventory(item_slot)) => {
            let hotbar_slot = &mut player_hotbar.pages[hotbar_index.0][hotbar_index.1];
            *hotbar_slot = Some(HotbarSlot::Inventory(item_slot));
        }
        Some(DragAndDropId::Skill(skill_slot)) => {
            let hotbar_slot = &mut player_hotbar.pages[hotbar_index.0][hotbar_index.1];
            *hotbar_slot = Some(HotbarSlot::Skill(skill_slot));
        }
        None => {}
    }
    /*
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
    */
}

pub fn ui_hotbar_system(
    mut egui_context: ResMut<EguiContext>,
    mut ui_state_hot_bar: Local<UiStateHotBar>,
    mut ui_state_dnd: ResMut<UiStateDragAndDrop>,
    mut query_player: Query<
        (&mut Hotbar, &Equipment, &Inventory, &SkillList),
        With<PlayerCharacter>,
    >,
    game_data: Res<GameData>,
    icons: Res<Icons>,
) {
    let (mut player_hotbar, player_equipment, player_inventory, player_skill_list) =
        query_player.single_mut();

    egui::Window::new("Hot Bar")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -10.0])
        .show(egui_context.ctx_mut(), |ui| {
            ui.horizontal(|ui| {
                egui::Grid::new("my_grid")
                    .spacing([4.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        let current_page = ui_state_hot_bar.current_page;

                        for i in 0..HOTBAR_PAGE_SIZE {
                            let hotbar_index = (current_page, i);
                            ui_add_hotbar_slot(
                                ui,
                                hotbar_index,
                                &mut player_hotbar,
                                player_equipment,
                                player_inventory,
                                player_skill_list,
                                &game_data,
                                &icons,
                                &mut ui_state_dnd,
                            );
                        }
                        ui.end_row();

                        for i in 1..=HOTBAR_PAGE_SIZE {
                            ui.vertical_centered_justified(|ui| {
                                ui.label(format!("{}", i));
                            });
                        }
                        ui.end_row();
                    });

                ui.vertical_centered(|ui| {
                    if ui.button("⬆").clicked() {
                        if ui_state_hot_bar.current_page == 0 {
                            ui_state_hot_bar.current_page = HOTBAR_NUM_PAGES - 1;
                        } else {
                            ui_state_hot_bar.current_page =
                                (ui_state_hot_bar.current_page - 1) % HOTBAR_NUM_PAGES;
                        }
                    }

                    ui.label(format!("{}", ui_state_hot_bar.current_page));

                    if ui.button("⬇").clicked() {
                        ui_state_hot_bar.current_page =
                            (ui_state_hot_bar.current_page + 1) % HOTBAR_NUM_PAGES;
                    }
                });
            });
        });
}
