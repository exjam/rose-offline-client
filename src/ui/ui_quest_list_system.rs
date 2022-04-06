use bevy::prelude::{Local, Query, Res, ResMut, With};
use bevy_egui::{egui, EguiContext};

use rose_game_common::components::QuestState;

use crate::{components::PlayerCharacter, resources::GameData, ui::UiStateWindows};

#[derive(Default)]
pub struct UiStateQuestList {
    current_quest_index: Option<usize>,
}

pub fn ui_quest_list_system(
    mut egui_context: ResMut<EguiContext>,
    mut ui_state_quest_list: Local<UiStateQuestList>,
    mut ui_state_windows: ResMut<UiStateWindows>,
    query_player: Query<&QuestState, With<PlayerCharacter>>,
    game_data: Res<GameData>,
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
                    egui::Grid::new("quest_list_grid")
                        .num_columns(1)
                        .spacing([4.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            for i in 0..player_quest_state.active_quests.len() {
                                if let Some(active_quest) =
                                    player_quest_state.active_quests[i].as_ref()
                                {
                                    let quest_data =
                                        game_data.quests.get_quest_data(active_quest.quest_id);
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
                                        ui.label(quest_description.unwrap_or(""));
                                        ui.separator();
                                    }
                                }
                            }
                        });
                });
        });
}
