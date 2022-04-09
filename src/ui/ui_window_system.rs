use bevy::prelude::{Res, ResMut};
use bevy_egui::{egui, EguiContext};

use crate::resources::Icons;

pub struct UiStateWindows {
    pub character_info_window_id: egui::Id,
    pub character_info_open: bool,

    pub inventory_window_id: egui::Id,
    pub inventory_open: bool,

    pub skill_list_window_id: egui::Id,
    pub skill_list_open: bool,

    pub quest_list_window_id: egui::Id,
    pub quest_list_open: bool,
}

impl Default for UiStateWindows {
    fn default() -> Self {
        Self {
            character_info_window_id: egui::Id::new("window_id_character_info"),
            character_info_open: false,
            inventory_window_id: egui::Id::new("window_id_inventory"),
            inventory_open: false,
            skill_list_window_id: egui::Id::new("window_id_skill_list"),
            skill_list_open: false,
            quest_list_window_id: egui::Id::new("window_id_quest_list"),
            quest_list_open: false,
        }
    }
}

pub fn ui_window_system(
    mut egui_context: ResMut<EguiContext>,
    mut ui_state_windows: ResMut<UiStateWindows>,
    icons: Res<Icons>,
) {
    if !egui_context.ctx_mut().wants_keyboard_input() {
        let mut input = egui_context.ctx_mut().input_mut();

        if input.consume_key(egui::Modifiers::ALT, egui::Key::A) {
            ui_state_windows.character_info_open = !ui_state_windows.character_info_open;
        }

        if input.consume_key(egui::Modifiers::ALT, egui::Key::I) {
            ui_state_windows.inventory_open = !ui_state_windows.inventory_open;
        }

        if input.consume_key(egui::Modifiers::ALT, egui::Key::S) {
            ui_state_windows.skill_list_open = !ui_state_windows.skill_list_open;
        }

        if input.consume_key(egui::Modifiers::ALT, egui::Key::Q) {
            ui_state_windows.quest_list_open = !ui_state_windows.quest_list_open;
        }
    }

    egui::Window::new("System Bar")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::RIGHT_CENTER, [-10.0, 0.0])
        .show(egui_context.ctx_mut(), |ui| {
            let icon_size = egui::Vec2::new(40.0, 40.0);
            let (texture_id, uv) = icons.get_window_icon_character_info();
            if ui
                .add(egui::ImageButton::new(texture_id, icon_size).uv(uv))
                .on_hover_text("Character (Alt + A)")
                .clicked()
            {
                ui_state_windows.character_info_open = !ui_state_windows.character_info_open;

                if ui_state_windows.character_info_open {
                    ui.ctx().move_to_top(egui::LayerId::new(
                        egui::Order::Middle,
                        ui_state_windows.character_info_window_id,
                    ));
                }
            }

            let (texture_id, uv) = icons.get_window_icon_inventory();
            if ui
                .add(egui::ImageButton::new(texture_id, icon_size).uv(uv))
                .on_hover_text("Inventory (Alt + I)")
                .clicked()
            {
                ui_state_windows.inventory_open = !ui_state_windows.inventory_open;

                if ui_state_windows.inventory_open {
                    ui.ctx().move_to_top(egui::LayerId::new(
                        egui::Order::Middle,
                        ui_state_windows.inventory_window_id,
                    ));
                }
            }

            let (texture_id, uv) = icons.get_window_icon_skills();
            if ui
                .add(egui::ImageButton::new(texture_id, icon_size).uv(uv))
                .on_hover_text("Skill List (Alt + S)")
                .clicked()
            {
                ui_state_windows.skill_list_open = !ui_state_windows.skill_list_open;

                if ui_state_windows.skill_list_open {
                    ui.ctx().move_to_top(egui::LayerId::new(
                        egui::Order::Middle,
                        ui_state_windows.skill_list_window_id,
                    ));
                }
            }

            let (texture_id, uv) = icons.get_window_icon_quests();
            if ui
                .add(egui::ImageButton::new(texture_id, icon_size).uv(uv))
                .on_hover_text("Quest List (Alt + Q)")
                .clicked()
            {
                ui_state_windows.quest_list_open = !ui_state_windows.quest_list_open;

                if ui_state_windows.quest_list_open {
                    ui.ctx().move_to_top(egui::LayerId::new(
                        egui::Order::Middle,
                        ui_state_windows.quest_list_window_id,
                    ));
                }
            }
        });
}
