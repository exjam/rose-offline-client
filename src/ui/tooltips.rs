use bevy_inspector_egui::egui;
use rose_data::{Item, SkillId};

use crate::resources::GameData;

pub fn ui_add_item_tooltip(ui: &mut egui::Ui, game_data: &GameData, item: &Item) {
    let item_data = game_data.items.get_base_item(item.get_item_reference());
    if item_data.is_none() {
        ui.label(format!(
            "Unknown Item\nItem Type: {:?} Item ID: {}",
            item.get_item_type(),
            item.get_item_number()
        ));
        return;
    }
    let item_data = item_data.unwrap();

    ui.label(format!(
        "{}\nItem Type: {:?} Item ID: {}",
        item_data.name,
        item.get_item_type(),
        item.get_item_number()
    ));
}

pub fn ui_add_skill_tooltip(ui: &mut egui::Ui, game_data: &GameData, skill_id: SkillId) {
    let skill_data = game_data.skills.get_skill(skill_id);
    if skill_data.is_none() {
        ui.label(format!("Unknown Skill\nSkill ID: {}", skill_id.get()));
        return;
    }
    let skill_data = skill_data.unwrap();

    ui.label(format!("{}\nSkill ID: {}", skill_data.name, skill_id.get()));
}
