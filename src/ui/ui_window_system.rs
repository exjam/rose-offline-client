use bevy::prelude::ResMut;
use bevy_egui::{egui, EguiContext};

pub struct UiStateWindows {
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
) {
    if !egui_context.ctx_mut().wants_keyboard_input() {
        let mut input = egui_context.ctx_mut().input_mut();

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
            if let Some(text_style) = ui.style_mut().text_styles.get_mut(&egui::TextStyle::Button) {
                text_style.size = 30.0;
            }
            ui.spacing_mut().item_spacing.y = 10.0;

            if ui
                .button("🛄")
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

            if ui
                .button("📖")
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

            if ui
                .button("Ｑ")
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
