use bevy::prelude::{Query, Res, ResMut, With};
use bevy_egui::{egui, EguiContext};

use rose_game_common::components::QuestState;

use crate::{
    components::PlayerCharacter,
    resources::{GameData, Icons},
    ui::UiStateWindows,
};

use super::{DragAndDropId, DragAndDropSlot};

fn ui_add_quest_item_slot(
    ui: &mut egui::Ui,
    quest_slot: usize,
    quest_item_index: usize,
    player_quest_state: &QuestState,
    game_data: &GameData,
    icons: &Icons,
) {
    let item = player_quest_state.active_quests[quest_slot]
        .as_ref()
        .and_then(|active_quest| active_quest.items[quest_item_index].as_ref());
    let item_data = item.and_then(|item| game_data.items.get_base_item(item.get_item_reference()));
    let contents =
        item_data.and_then(|item_data| icons.get_item_icon(item_data.icon_index as usize));
    let quantity = item.and_then(|item| {
        if item.get_item_type().is_stackable_item() {
            Some(item.get_quantity() as usize)
        } else {
            None
        }
    });

    let mut dragged_item = None;
    let mut dropped_item = None;
    let response = ui.add(DragAndDropSlot::new(
        DragAndDropId::NotDraggable,
        contents,
        quantity,
        |_| false,
        &mut dragged_item,
        &mut dropped_item,
        [40.0, 40.0],
    ));

    if let (Some(item), Some(item_data)) = (item, item_data) {
        response.on_hover_text(format!(
            "{}\nItem Type: {:?} Item ID: {}",
            item_data.name,
            item.get_item_type(),
            item.get_item_number()
        ));
    }
}

pub fn ui_quest_list_system(
    mut egui_context: ResMut<EguiContext>,
    mut ui_state_windows: ResMut<UiStateWindows>,
    query_player: Query<&QuestState, With<PlayerCharacter>>,
    game_data: Res<GameData>,
    icons: Res<Icons>,
) {
    let player_quest_state = query_player.single();

    egui::Window::new("Quest List")
        .id(ui_state_windows.quest_list_window_id)
        .open(&mut ui_state_windows.quest_list_open)
        .resizable(true)
        .show(egui_context.ctx_mut(), |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .always_show_scroll(true)
                .show(ui, |ui| {
                    for i in 0..player_quest_state.active_quests.len() {
                        if let Some(active_quest) = player_quest_state.active_quests[i].as_ref() {
                            let quest_data = game_data.quests.get_quest_data(active_quest.quest_id);
                            let quest_name = quest_data.and_then(|quest_data| {
                                game_data
                                    .stl_quest
                                    .get_text_string(1, &quest_data.string_id)
                            });
                            let quest_description = quest_data.and_then(|quest_data| {
                                game_data
                                    .stl_quest
                                    .get_comment_string(1, &quest_data.string_id)
                            });

                            if let Some(quest_name) = quest_name {
                                ui.heading(quest_name);
                            } else {
                                ui.heading(format!("Quest ID: {}", active_quest.quest_id));
                            }

                            ui.label(quest_description.unwrap_or(""));

                            ui.horizontal(|ui| {
                                for j in 0..active_quest.items.len() {
                                    ui_add_quest_item_slot(
                                        ui,
                                        i,
                                        j,
                                        player_quest_state,
                                        &game_data,
                                        &icons,
                                    );
                                }
                            });

                            ui.separator();
                        }
                    }
                });
        });
}
