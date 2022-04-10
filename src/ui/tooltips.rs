use std::cmp::Ordering;

use bevy_inspector_egui::egui;

use rose_data::{
    BaseItemData, EquipmentItem, Item, ItemClass, ItemGradeData, ItemType, SkillId, StackableItem,
};

use crate::resources::GameData;

fn get_item_name_color(_item_data: &BaseItemData) -> egui::Color32 {
    // TODO: Get correct item color from stb
    egui::Color32::YELLOW
}

fn add_equipment_item_name(
    ui: &mut egui::Ui,
    equipment_item: &EquipmentItem,
    item_data: &BaseItemData,
) {
    if equipment_item.grade > 0 {
        ui.colored_label(
            get_item_name_color(&item_data),
            format!("{} ({})", &item_data.name, equipment_item.grade),
        );
    } else {
        ui.colored_label(get_item_name_color(&item_data), &item_data.name);
    }
}

fn add_stackable_item_name(
    ui: &mut egui::Ui,
    _stackable_item: &StackableItem,
    item_data: &BaseItemData,
) {
    ui.colored_label(get_item_name_color(&item_data), &item_data.name);
}

fn add_equipment_item_life_durability(ui: &mut egui::Ui, equipment_item: &EquipmentItem) {
    ui.label(format!(
        "Life: {:?}% Durability: {}",
        (equipment_item.life + 9) / 10,
        equipment_item.durability
    ));
}

fn add_item_defence(
    ui: &mut egui::Ui,
    item_data: &BaseItemData,
    grade_data: Option<&ItemGradeData>,
) {
    ui.label(format!(
        "Defence: {} Magic Resistance: {}",
        item_data.defence
            + grade_data
                .map(|grade_data| grade_data.defence as u32)
                .unwrap_or(0),
        item_data.resistance
            + grade_data
                .map(|grade_data| grade_data.resistance as u32)
                .unwrap_or(0)
    ));
}

fn add_item_add_ability(ui: &mut egui::Ui, item_data: &BaseItemData) {
    for &(ability_type, value) in item_data.add_ability.iter() {
        ui.colored_label(
            egui::Color32::from_rgb(100, 200, 255),
            format!("[{:?} {}]", ability_type, value),
        );
    }
}

fn add_equipment_item_add_appraisal(
    ui: &mut egui::Ui,
    game_data: &GameData,
    equipment_item: &EquipmentItem,
) {
    if equipment_item.gem == 0 {
        return;
    }

    let is_gem = equipment_item.gem > 300;
    if !is_gem && !equipment_item.is_appraised {
        ui.colored_label(egui::Color32::RED, "[Requires Appraisal]");
    } else if let Some(gem_item_data) = game_data.items.get_gem_item(equipment_item.gem as usize) {
        if is_gem {
            ui.colored_label(egui::Color32::YELLOW, &gem_item_data.item_data.name);
        }

        for &(ability_type, value) in gem_item_data.gem_add_ability.iter() {
            ui.colored_label(
                if is_gem {
                    egui::Color32::YELLOW
                } else {
                    egui::Color32::from_rgb(100, 200, 255)
                },
                format!("[{:?} {}]", ability_type, value),
            );
        }
    }
}

fn add_item_equip_requirement(ui: &mut egui::Ui, item_data: &BaseItemData) {
    if item_data.equip_class_requirement != 0 {
        // TODO: Class requirement strings
        ui.colored_label(
            egui::Color32::GREEN,
            format!("[Class Requirement {}]", item_data.equip_class_requirement),
        );
    }

    for union_id in item_data.equip_union_requirement.iter() {
        // TODO: Union names
        ui.colored_label(
            egui::Color32::GREEN,
            format!("[Union Requirement {}]", union_id),
        );
    }

    for &(ability_type, value) in item_data.equip_ability_requirement.iter() {
        // TODO: Check if ability requirements are met
        ui.colored_label(
            egui::Color32::GREEN,
            format!("[{:?} {}]", ability_type, value),
        );
    }
}

fn add_item_description(ui: &mut egui::Ui, item_data: &BaseItemData) {
    ui.label(format!("Weight: {}", item_data.weight));

    // TODO: add_item_description
}

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

    match item {
        Item::Equipment(equipment_item) => {
            add_equipment_item_name(ui, equipment_item, item_data);

            match equipment_item.item.item_type {
                ItemType::Weapon => {
                    let weapon_item_data = game_data
                        .items
                        .get_weapon_item(equipment_item.item.item_number)
                        .unwrap();
                    let grade_data = game_data.items.get_item_grade(equipment_item.grade);

                    let hit_rate = item_data.quality as f32 * 0.6
                        + equipment_item.durability as f32 * 0.8
                        + grade_data.map(|grade| grade.hit).unwrap_or(0) as f32;

                    ui.label(format!(
                        "Item Class: {:?} Hit Rate: {}",
                        item_data.class, hit_rate as i32
                    ));

                    add_equipment_item_life_durability(ui, equipment_item);

                    let attack_power = weapon_item_data.attack_power
                        + grade_data.map(|grade| grade.attack).unwrap_or(0);
                    match weapon_item_data.attack_speed.cmp(&12) {
                        Ordering::Less => {
                            ui.label(format!(
                                "Attack Power: {} Attack Speed: Fast +{}",
                                attack_power,
                                12 - weapon_item_data.attack_speed
                            ));
                        }
                        Ordering::Equal => {
                            ui.label(format!(
                                "Attack Power: {} Attack Speed: Normal",
                                attack_power
                            ));
                        }
                        Ordering::Greater => {
                            ui.label(format!(
                                "Attack Power: {} Attack Speed: Slow -{}",
                                attack_power,
                                weapon_item_data.attack_speed - 12
                            ));
                        }
                    }

                    ui.label(format!(
                        "Attack Range: {}M",
                        weapon_item_data.attack_range / 100
                    ));

                    add_item_add_ability(ui, item_data);
                    add_equipment_item_add_appraisal(ui, game_data, equipment_item);
                    add_item_equip_requirement(ui, item_data);
                    add_item_description(ui, item_data);
                }
                ItemType::SubWeapon => {
                    let grade_data = game_data.items.get_item_grade(equipment_item.grade);

                    if matches!(item_data.class, ItemClass::Shield) {
                        let avoid_rate = equipment_item.durability as f32 * 0.3
                            + grade_data.map(|grade| grade.avoid).unwrap_or(0) as f32;

                        ui.label(format!(
                            "Item Class: {:?} Avoid Rate: {}",
                            item_data.class, avoid_rate as i32
                        ));
                    } else {
                        ui.label(format!(
                            "Item Class: {:?} Quality: {}",
                            item_data.class, item_data.quality
                        ));
                    }

                    add_equipment_item_life_durability(ui, equipment_item);

                    if matches!(item_data.class, ItemClass::Shield) {
                        add_item_defence(ui, item_data, grade_data);
                    }

                    add_item_add_ability(ui, item_data);
                    add_equipment_item_add_appraisal(ui, game_data, equipment_item);
                    add_item_equip_requirement(ui, item_data);
                    add_item_description(ui, item_data);
                }
                ItemType::Face
                | ItemType::Head
                | ItemType::Body
                | ItemType::Hands
                | ItemType::Feet
                | ItemType::Back => {
                    let grade_data = game_data.items.get_item_grade(equipment_item.grade);

                    if matches!(equipment_item.item.item_type, ItemType::Face) {
                        ui.label(format!(
                            "Item Class: {:?} Quality: {}",
                            item_data.class, item_data.quality
                        ));
                    } else {
                        let avoid_rate = equipment_item.durability as f32 * 0.3
                            + grade_data.map(|grade| grade.avoid).unwrap_or(0) as f32;

                        ui.label(format!(
                            "Item Class: {:?} Avoid Rate: {}",
                            item_data.class, avoid_rate as i32
                        ));
                    }

                    add_equipment_item_life_durability(ui, equipment_item);
                    add_item_defence(ui, item_data, grade_data);

                    if matches!(equipment_item.item.item_type, ItemType::Feet) {
                        if let Some(move_speed) = game_data
                            .items
                            .get_feet_item(equipment_item.item.item_number)
                            .map(|feet_item_data| feet_item_data.move_speed)
                        {
                            ui.label(format!("[Movement Speed {}]", move_speed));
                        }
                    } else if matches!(equipment_item.item.item_type, ItemType::Back) {
                        if let Some(move_speed) = game_data
                            .items
                            .get_back_item(equipment_item.item.item_number)
                            .map(|back_item_data| back_item_data.move_speed)
                        {
                            ui.label(format!("[Movement Speed {}]", move_speed));
                        }
                    }

                    add_item_add_ability(ui, item_data);
                    add_equipment_item_add_appraisal(ui, game_data, equipment_item);
                    add_item_equip_requirement(ui, item_data);
                    add_item_description(ui, item_data);
                }
                ItemType::Jewellery => {
                    ui.label(format!(
                        "Item Class: {:?} Quality: {}",
                        item_data.class, item_data.quality
                    ));

                    add_item_add_ability(ui, item_data);
                    add_equipment_item_add_appraisal(ui, game_data, equipment_item);
                    add_item_equip_requirement(ui, item_data);
                    add_item_description(ui, item_data);
                }
                ItemType::Vehicle => {
                    ui.label(format!(
                        "Item Class: {:?} Quality: {}",
                        item_data.class, item_data.quality
                    ));

                    // TODO: Vehicle tooltip
                    add_item_description(ui, item_data);
                }
                _ => panic!("Unexpected item type"),
            }
        }
        Item::Stackable(stackable_item) => {
            add_stackable_item_name(ui, stackable_item, item_data);

            match stackable_item.item.item_type {
                ItemType::Consumable => {
                    let use_item_data = game_data
                        .items
                        .get_consumable_item(stackable_item.item.item_number as usize);

                    ui.label(format!(
                        "Item Class: {:?} Quality: {}",
                        item_data.class, item_data.quality
                    ));

                    match item_data.class {
                        ItemClass::EngineFuel => {
                            // TODO: Tooltip for ItemClass::EngineFuel
                        }
                        ItemClass::SkillBook => {
                            // TODO: Tooltip for ItemClass::SkillBook
                        }
                        ItemClass::MagicItem => {
                            // TODO: Tooltip for ItemClass::MagicItem
                        }
                        ItemClass::RepairTool => {
                            // TODO: Tooltip for ItemClass::RepairTool
                        }
                        _ => {
                            if let Some(use_item_data) = use_item_data {
                                for &(ability_type, value) in use_item_data.add_ability.iter() {
                                    ui.label(format!("[{:?} {}]", ability_type, value));
                                }
                            }
                        }
                    }

                    add_item_description(ui, item_data);
                }
                ItemType::Gem => {
                    let gem_item_data = game_data
                        .items
                        .get_gem_item(stackable_item.item.item_number as usize);

                    ui.label(format!(
                        "Item Class: {:?} Quality: {}",
                        item_data.class, item_data.quality
                    ));

                    if let Some(gem_item_data) = gem_item_data {
                        for &(ability_type, value) in gem_item_data.gem_add_ability.iter() {
                            ui.colored_label(
                                egui::Color32::from_rgb(100, 200, 255),
                                format!("[{:?} {}]", ability_type, value),
                            );
                        }
                    }

                    add_item_description(ui, item_data);
                }
                ItemType::Material => {
                    ui.label(format!(
                        "Item Class: {:?} Quality: {}",
                        item_data.class, item_data.quality
                    ));

                    add_item_description(ui, item_data);
                }
                ItemType::Quest => {}
                _ => panic!("Unexpected item type"),
            }
        }
    }
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
