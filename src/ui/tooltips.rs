use std::cmp::Ordering;
use std::fmt::Write;

use bevy::{ecs::query::WorldQuery, prelude::Entity};
use bevy_egui::egui;

use rose_data::{
    AbilityType, BaseItemData, EquipmentItem, Item, ItemClass, ItemGradeData, ItemType, JobId,
    SkillAddAbility, SkillData, SkillId, SkillType, StackableItem, StatusEffectType,
};
use rose_game_common::components::{
    AbilityValues, CharacterInfo, Equipment, ExperiencePoints, HealthPoints, Inventory, Level,
    ManaPoints, MoveSpeed, SkillList, SkillPoints, Stamina, StatPoints, Team, UnionMembership,
};

use crate::{bundles::ability_values_get_value, resources::GameData};

const TOOLTIP_MAX_WIDTH: f32 = 300.0;

#[derive(WorldQuery)]
pub struct PlayerTooltipQuery<'w> {
    pub ability_values: &'w AbilityValues,
    pub character_info: &'w CharacterInfo,
    pub experience_points: &'w ExperiencePoints,
    pub health_points: &'w HealthPoints,
    pub equipment: &'w Equipment,
    pub inventory: &'w Inventory,
    pub level: &'w Level,
    pub mana_points: &'w ManaPoints,
    pub move_speed: &'w MoveSpeed,
    pub skill_list: &'w SkillList,
    pub skill_points: &'w SkillPoints,
    pub stamina: &'w Stamina,
    pub stat_points: &'w StatPoints,
    pub team: &'w Team,
    pub union_membership: &'w UnionMembership,
}

fn get_item_name_color(item_type: ItemType, item_data: &BaseItemData) -> egui::Color32 {
    match item_type {
        ItemType::Head
        | ItemType::Body
        | ItemType::Hands
        | ItemType::Feet
        | ItemType::Weapon
        | ItemType::SubWeapon => match item_data.rare_type {
            1..=20 => egui::Color32::from_rgb(0, 255, 255),
            21 => egui::Color32::from_rgb(255, 128, 255),
            _ => egui::Color32::YELLOW,
        },
        _ => egui::Color32::YELLOW,
    }
}

fn add_equipment_item_name(
    ui: &mut egui::Ui,
    equipment_item: &EquipmentItem,
    item_data: &BaseItemData,
) {
    let text = if equipment_item.grade > 0 {
        format!("{} ({})", &item_data.name, equipment_item.grade)
    } else {
        item_data.name.to_string()
    };

    ui.add(egui::Label::new(
        egui::RichText::new(text)
            .color(get_item_name_color(
                equipment_item.item.item_type,
                item_data,
            ))
            .font(egui::FontId::new(
                16.0,
                egui::FontFamily::Name("Ubuntu-M".into()),
            )),
    ));
}

fn add_stackable_item_name(
    ui: &mut egui::Ui,
    stackable_item: &StackableItem,
    item_data: &BaseItemData,
) {
    ui.add(egui::Label::new(
        egui::RichText::new(item_data.name)
            .color(get_item_name_color(
                stackable_item.item.item_type,
                item_data,
            ))
            .font(egui::FontId::new(
                16.0,
                egui::FontFamily::Name("Ubuntu-M".into()),
            )),
    ));
}

fn add_equipment_item_life_durability(
    ui: &mut egui::Ui,
    game_data: &GameData,
    equipment_item: &EquipmentItem,
) {
    ui.label(format!(
        "{}:{: >3}% {}:{: >3}",
        game_data.client_strings.item_life,
        (equipment_item.life + 9) / 10,
        game_data.client_strings.item_durability,
        equipment_item.durability
    ));
}

fn add_item_defence(
    ui: &mut egui::Ui,
    game_data: &GameData,
    item_data: &BaseItemData,
    grade_data: Option<&ItemGradeData>,
) {
    ui.label(format!(
        "{}:{} {}:{}",
        game_data
            .string_database
            .get_ability_type(AbilityType::Defence),
        item_data.defence
            + grade_data
                .map(|grade_data| grade_data.defence as u32)
                .unwrap_or(0),
        game_data
            .string_database
            .get_ability_type(AbilityType::Resistance),
        item_data.resistance
            + grade_data
                .map(|grade_data| grade_data.resistance as u32)
                .unwrap_or(0)
    ));
}

fn add_item_add_ability(ui: &mut egui::Ui, game_data: &GameData, item_data: &BaseItemData) {
    for &(ability_type, value) in item_data.add_ability.iter() {
        ui.colored_label(
            egui::Color32::from_rgb(100, 200, 255),
            format!(
                "[{} {}{}]",
                game_data.string_database.get_ability_type(ability_type),
                value,
                if matches!(ability_type, AbilityType::SaveMana) {
                    "%"
                } else {
                    ""
                }
            ),
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
        ui.colored_label(
            egui::Color32::RED,
            game_data.client_strings.item_requires_appraisal,
        );
    } else if let Some(gem_item_data) = game_data.items.get_gem_item(equipment_item.gem as usize) {
        if is_gem {
            ui.colored_label(egui::Color32::YELLOW, gem_item_data.item_data.name);
        }

        for &(ability_type, value) in gem_item_data.gem_add_ability.iter() {
            ui.colored_label(
                if is_gem {
                    egui::Color32::YELLOW
                } else {
                    egui::Color32::from_rgb(100, 200, 255)
                },
                format!(
                    "[{} {}{}]",
                    game_data.string_database.get_ability_type(ability_type),
                    value,
                    if matches!(ability_type, AbilityType::SaveMana) {
                        "%"
                    } else {
                        ""
                    }
                ),
            );
        }
    }
}

fn add_item_equip_requirement(
    ui: &mut egui::Ui,
    game_data: &GameData,
    player: Option<&PlayerTooltipQueryItem>,
    item_data: &BaseItemData,
) {
    if let Some(job_class_id) = item_data.equip_job_class_requirement {
        if let Some(job_class) = game_data.job_class.get(job_class_id) {
            let color = if player.map_or(true, |player| {
                job_class
                    .jobs
                    .contains(&JobId::new(player.character_info.job))
            }) {
                egui::Color32::GREEN
            } else {
                egui::Color32::RED
            };

            ui.colored_label(
                color,
                format!(
                    "[{}: {}]",
                    game_data.client_strings.equip_require_job, job_class.name
                ),
            );
        }
    }

    if !item_data.equip_union_requirement.is_empty() {
        let mut union_text = format!(
            "[{}:",
            game_data
                .string_database
                .get_ability_type(AbilityType::Union)
        );
        let mut union_color = egui::Color32::RED;
        for union_id in item_data.equip_union_requirement.iter() {
            if let Some(player) = player {
                if let Some(current_union) = player.union_membership.current_union {
                    if current_union.get() == *union_id as usize {
                        union_color = egui::Color32::GREEN;
                    }
                }
            }

            write!(&mut union_text, " {}", union_id).ok();
        }
        union_text.push(']');
        ui.colored_label(union_color, union_text);
    }

    for &(ability_type, value) in item_data.equip_ability_requirement.iter() {
        let mut color = egui::Color32::RED;

        if let Some(player) = player {
            if let Some(current_value) = ability_values_get_value(
                ability_type,
                player.ability_values,
                Some(player.character_info),
                Some(player.experience_points),
                Some(player.health_points),
                Some(player.inventory),
                Some(player.level),
                Some(player.mana_points),
                Some(player.move_speed),
                Some(player.skill_points),
                Some(player.stamina),
                Some(player.stat_points),
                Some(player.team),
                Some(player.union_membership),
            ) {
                if current_value >= value as i32 {
                    color = egui::Color32::GREEN;
                }
            }
        }

        ui.colored_label(
            color,
            format!(
                "[{} {}]",
                game_data.string_database.get_ability_type(ability_type),
                value
            ),
        );
    }
}

fn add_item_description(ui: &mut egui::Ui, game_data: &GameData, item_data: &BaseItemData) {
    ui.label(format!(
        "{}:{}",
        game_data.client_strings.item_weight, item_data.weight
    ));
    ui.label(item_data.description);
}

pub fn ui_add_item_tooltip(
    ui: &mut egui::Ui,
    game_data: &GameData,
    player: Option<&PlayerTooltipQueryItem>,
    item: &Item,
) {
    ui.set_max_width(TOOLTIP_MAX_WIDTH);
    ui.style_mut().visuals.widgets.noninteractive.fg_stroke =
        egui::Stroke::new(1.0, egui::Color32::WHITE);

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
                        "{}:{} {}:{}",
                        game_data.client_strings.item_class,
                        game_data.string_database.get_item_class(item_data.class),
                        game_data.string_database.get_ability_type(AbilityType::Hit),
                        hit_rate as i32
                    ));

                    add_equipment_item_life_durability(ui, game_data, equipment_item);

                    let attack_power = weapon_item_data.attack_power
                        + grade_data.map(|grade| grade.attack).unwrap_or(0);
                    match weapon_item_data.attack_speed.cmp(&12) {
                        Ordering::Less => {
                            ui.label(format!(
                                "{}:{} {}:{} +{}",
                                game_data
                                    .string_database
                                    .get_ability_type(AbilityType::Attack),
                                attack_power,
                                game_data
                                    .string_database
                                    .get_ability_type(AbilityType::AttackSpeed),
                                game_data.client_strings.item_attack_speed_fast,
                                12 - weapon_item_data.attack_speed
                            ));
                        }
                        Ordering::Equal => {
                            ui.label(format!(
                                "{}:{} {}:{}",
                                game_data
                                    .string_database
                                    .get_ability_type(AbilityType::Attack),
                                attack_power,
                                game_data
                                    .string_database
                                    .get_ability_type(AbilityType::AttackSpeed),
                                game_data.client_strings.item_attack_speed_normal,
                            ));
                        }
                        Ordering::Greater => {
                            ui.label(format!(
                                "{}:{} {}:{} -{}",
                                game_data
                                    .string_database
                                    .get_ability_type(AbilityType::Attack),
                                attack_power,
                                game_data
                                    .string_database
                                    .get_ability_type(AbilityType::AttackSpeed),
                                game_data.client_strings.item_attack_speed_slow,
                                weapon_item_data.attack_speed - 12
                            ));
                        }
                    }

                    ui.label(format!(
                        "{}:{}M",
                        game_data.client_strings.item_attack_range,
                        weapon_item_data.attack_range / 100
                    ));

                    add_item_add_ability(ui, game_data, item_data);
                    add_equipment_item_add_appraisal(ui, game_data, equipment_item);
                    add_item_equip_requirement(ui, game_data, player, item_data);
                    add_item_description(ui, game_data, item_data);
                }
                ItemType::SubWeapon => {
                    let grade_data = game_data.items.get_item_grade(equipment_item.grade);

                    if matches!(item_data.class, ItemClass::Shield) {
                        let avoid_rate = equipment_item.durability as f32 * 0.3
                            + grade_data.map(|grade| grade.avoid).unwrap_or(0) as f32;

                        ui.label(format!(
                            "{}:{} {}:{}",
                            game_data.client_strings.item_class,
                            game_data.string_database.get_item_class(item_data.class),
                            game_data
                                .string_database
                                .get_ability_type(AbilityType::Avoid),
                            avoid_rate as i32
                        ));
                    } else {
                        ui.label(format!(
                            "{}:{} {}:{}",
                            game_data.client_strings.item_class,
                            game_data.string_database.get_item_class(item_data.class),
                            game_data.client_strings.item_quality,
                            item_data.quality
                        ));
                    }

                    add_equipment_item_life_durability(ui, game_data, equipment_item);

                    if matches!(item_data.class, ItemClass::Shield) {
                        add_item_defence(ui, game_data, item_data, grade_data);
                    }

                    add_item_add_ability(ui, game_data, item_data);
                    add_equipment_item_add_appraisal(ui, game_data, equipment_item);
                    add_item_equip_requirement(ui, game_data, player, item_data);
                    add_item_description(ui, game_data, item_data);
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
                            "{}:{} {}:{}",
                            game_data.client_strings.item_class,
                            game_data.string_database.get_item_class(item_data.class),
                            game_data.client_strings.item_quality,
                            item_data.quality
                        ));
                    } else {
                        let avoid_rate = equipment_item.durability as f32 * 0.3
                            + grade_data.map(|grade| grade.avoid).unwrap_or(0) as f32;

                        ui.label(format!(
                            "{}:{} {}:{}",
                            game_data.client_strings.item_class,
                            game_data.string_database.get_item_class(item_data.class),
                            game_data
                                .string_database
                                .get_ability_type(AbilityType::Avoid),
                            avoid_rate as i32
                        ));
                    }

                    add_equipment_item_life_durability(ui, game_data, equipment_item);
                    add_item_defence(ui, game_data, item_data, grade_data);

                    if matches!(equipment_item.item.item_type, ItemType::Feet) {
                        if let Some(move_speed) = game_data
                            .items
                            .get_feet_item(equipment_item.item.item_number)
                            .map(|feet_item_data| feet_item_data.move_speed)
                        {
                            ui.label(format!(
                                "[{} {}]",
                                game_data.client_strings.item_move_speed, move_speed
                            ));
                        }
                    } else if matches!(equipment_item.item.item_type, ItemType::Back) {
                        if let Some(move_speed) = game_data
                            .items
                            .get_back_item(equipment_item.item.item_number)
                            .map(|back_item_data| back_item_data.move_speed)
                        {
                            ui.label(format!(
                                "[{} {}]",
                                game_data.client_strings.item_move_speed, move_speed
                            ));
                        }
                    }

                    add_item_add_ability(ui, game_data, item_data);
                    add_equipment_item_add_appraisal(ui, game_data, equipment_item);
                    add_item_equip_requirement(ui, game_data, player, item_data);
                    add_item_description(ui, game_data, item_data);
                }
                ItemType::Jewellery => {
                    ui.label(format!(
                        "{}:{} {}:{}",
                        game_data.client_strings.item_class,
                        game_data.string_database.get_item_class(item_data.class),
                        game_data.client_strings.item_quality,
                        item_data.quality
                    ));

                    add_item_add_ability(ui, game_data, item_data);
                    add_equipment_item_add_appraisal(ui, game_data, equipment_item);
                    add_item_equip_requirement(ui, game_data, player, item_data);
                    add_item_description(ui, game_data, item_data);
                }
                ItemType::Vehicle => {
                    ui.label(format!(
                        "{}:{} {}:{}",
                        game_data.client_strings.item_class,
                        game_data.string_database.get_item_class(item_data.class),
                        game_data.client_strings.item_quality,
                        item_data.quality
                    ));

                    // TODO: Vehicle tooltip
                    add_item_description(ui, game_data, item_data);
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
                        "{}:{} {}:{}",
                        game_data.client_strings.item_class,
                        game_data.string_database.get_item_class(item_data.class),
                        game_data.client_strings.item_quality,
                        item_data.quality
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
                                if let Some((ability_type, value)) =
                                    use_item_data.add_ability.as_ref()
                                {
                                    ui.label(format!("[{:?} {}]", ability_type, value));
                                }
                            }
                        }
                    }

                    add_item_description(ui, game_data, item_data);
                }
                ItemType::Gem => {
                    let gem_item_data = game_data
                        .items
                        .get_gem_item(stackable_item.item.item_number as usize);

                    ui.label(format!(
                        "{}:{} {}:{}",
                        game_data.client_strings.item_class,
                        game_data.string_database.get_item_class(item_data.class),
                        game_data.client_strings.item_quality,
                        item_data.quality
                    ));

                    if let Some(gem_item_data) = gem_item_data {
                        for &(ability_type, value) in gem_item_data.gem_add_ability.iter() {
                            ui.colored_label(
                                egui::Color32::from_rgb(100, 200, 255),
                                format!(
                                    "[{} {}{}]",
                                    game_data.string_database.get_ability_type(ability_type),
                                    value,
                                    if matches!(ability_type, AbilityType::SaveMana) {
                                        "%"
                                    } else {
                                        ""
                                    }
                                ),
                            );
                        }
                    }

                    add_item_description(ui, game_data, item_data);
                }
                ItemType::Material => {
                    ui.label(format!(
                        "{}:{} {}:{}",
                        game_data.client_strings.item_class,
                        game_data.string_database.get_item_class(item_data.class),
                        game_data.client_strings.item_quality,
                        item_data.quality
                    ));

                    add_item_description(ui, game_data, item_data);
                }
                ItemType::Quest => {}
                _ => panic!("Unexpected item type"),
            }
        }
    }
}

fn add_skill_name(ui: &mut egui::Ui, game_data: &GameData, skill_data: &SkillData) {
    let text = if skill_data.name.is_empty() {
        format!("??? [Skill ID: {}]", skill_data.id.get())
    } else if skill_data.level > 1 {
        format!(
            "{} [{}: {}]",
            &skill_data.name, game_data.client_strings.skill_level, skill_data.level
        )
    } else {
        skill_data.name.to_string()
    };

    ui.add(egui::Label::new(
        egui::RichText::new(text)
            .color(egui::Color32::YELLOW)
            .font(egui::FontId::new(
                16.0,
                egui::FontFamily::Name("Ubuntu-M".into()),
            )),
    ));
}

fn add_skill_next_level<'a>(
    ui: &mut egui::Ui,
    game_data: &'a GameData,
    skill_data: &SkillData,
) -> Option<&'a SkillData> {
    let next_level_skill_data = game_data
        .skills
        .get_skill(SkillId::new(skill_data.id.get() + 1).unwrap())?;
    if next_level_skill_data.base_skill_id != skill_data.base_skill_id
        || next_level_skill_data.level != skill_data.level + 1
    {
        return None;
    }

    let name = if next_level_skill_data.name.is_empty() {
        format!("??? [Skill ID: {}]", next_level_skill_data.id.get())
    } else if next_level_skill_data.level > 1 {
        format!(
            "{} [{}: {}]",
            &next_level_skill_data.name,
            game_data.client_strings.skill_level,
            next_level_skill_data.level
        )
    } else {
        next_level_skill_data.name.to_string()
    };

    ui.separator();
    ui.label(
        egui::RichText::new(format!(
            "{}: {}",
            game_data.client_strings.skill_next_level_info, name
        ))
        .color(egui::Color32::YELLOW)
        .font(egui::FontId::new(
            16.0,
            egui::FontFamily::Name("Ubuntu-M".into()),
        )),
    );

    Some(next_level_skill_data)
}

fn add_skill_aoe_range(ui: &mut egui::Ui, game_data: &GameData, skill_data: &SkillData) {
    if skill_data.scope > 0 {
        ui.label(format!(
            "{}: {}m",
            game_data.client_strings.skill_aoe_range,
            skill_data.scope / 100
        ));
    }
}

fn add_skill_cast_range(ui: &mut egui::Ui, game_data: &GameData, skill_data: &SkillData) {
    if skill_data.cast_range > 0 {
        ui.label(format!(
            "{}: {}m",
            game_data.client_strings.skill_cast_range,
            skill_data.cast_range / 100
        ));
    }
}

fn add_skill_description(ui: &mut egui::Ui, skill_data: &SkillData) {
    ui.allocate_at_least(
        egui::vec2(ui.available_size_before_wrap().x, 6.0),
        egui::Sense::hover(),
    );
    ui.label(skill_data.description);
}

fn add_skill_power(ui: &mut egui::Ui, game_data: &GameData, skill_data: &SkillData) {
    let damage_type = match skill_data.damage_type {
        0 => game_data.client_strings.skill_damage_type_0,
        1 => game_data.client_strings.skill_damage_type_1,
        2 => game_data.client_strings.skill_damage_type_2,
        3 => game_data.client_strings.skill_damage_type_3,
        _ => "",
    };

    ui.label(format!(
        "{}: {} ({})",
        game_data.client_strings.skill_power, damage_type, skill_data.power
    ));
}

fn add_skill_recover_xp(ui: &mut egui::Ui, game_data: &GameData, skill_data: &SkillData) {
    ui.label(format!(
        "{}: {}%",
        game_data.client_strings.skill_recover_xp, skill_data.power
    ));
}

fn add_skill_require_ability(
    ui: &mut egui::Ui,
    game_data: &GameData,
    player: Option<&PlayerTooltipQueryItem>,
    skill_data: &SkillData,
) {
    if skill_data.required_ability.is_empty() {
        return;
    }

    for &(ability_type, value) in skill_data.required_ability.iter() {
        let mut color = egui::Color32::RED;

        if let Some(player) = player {
            if let Some(current_value) = ability_values_get_value(
                ability_type,
                player.ability_values,
                Some(player.character_info),
                Some(player.experience_points),
                Some(player.health_points),
                Some(player.inventory),
                Some(player.level),
                Some(player.mana_points),
                Some(player.move_speed),
                Some(player.skill_points),
                Some(player.stamina),
                Some(player.stat_points),
                Some(player.team),
                Some(player.union_membership),
            ) {
                if current_value >= value as i32 {
                    color = egui::Color32::GREEN;
                }
            }
        }
        ui.colored_label(
            color,
            format!(
                "[{}: {} {}]",
                game_data.client_strings.skill_require_ability,
                game_data.string_database.get_ability_type(ability_type),
                value
            ),
        );
    }
}

fn add_skill_require_job(
    ui: &mut egui::Ui,
    game_data: &GameData,
    player: Option<&PlayerTooltipQueryItem>,
    skill_data: &SkillData,
) {
    if let Some(job_class_id) = skill_data.required_job_class {
        if let Some(job_class) = game_data.job_class.get(job_class_id) {
            let color = if player.map_or(true, |player| {
                job_class
                    .jobs
                    .contains(&JobId::new(player.character_info.job))
            }) {
                egui::Color32::GREEN
            } else {
                egui::Color32::RED
            };

            ui.colored_label(
                color,
                format!(
                    "[{}: {}]",
                    game_data.client_strings.skill_require_job, job_class.name
                ),
            );
        }
    }
}

fn add_skill_require_skill(
    ui: &mut egui::Ui,
    game_data: &GameData,
    player: Option<&PlayerTooltipQueryItem>,
    skill_data: &SkillData,
) {
    if skill_data.required_skills.is_empty() {
        return;
    }

    for &(required_skill_id, required_level) in skill_data.required_skills.iter() {
        if let Some(required_skill_data) = game_data.skills.get_skill(
            SkillId::new(required_skill_id.get() + required_level.max(1) as u16 - 1).unwrap(),
        ) {
            let mut color = egui::Color32::RED;

            if let Some(player) = player {
                if let Some((_, _, skill_level)) = player.skill_list.find_skill_level(
                    &game_data.skills,
                    required_skill_data
                        .base_skill_id
                        .unwrap_or(required_skill_id),
                ) {
                    if skill_level >= required_level as u32 {
                        color = egui::Color32::GREEN;
                    }
                }
            }

            ui.colored_label(
                color,
                format!(
                    "[{}: {} ({}: {})]",
                    game_data.client_strings.skill_require_skill,
                    required_skill_data.name,
                    game_data
                        .string_database
                        .get_ability_type(AbilityType::Level),
                    required_level
                ),
            );
        }
    }
}

fn add_skill_require_skill_point(
    ui: &mut egui::Ui,
    game_data: &GameData,
    player: Option<&PlayerTooltipQueryItem>,
    skill_data: &SkillData,
) {
    if skill_data.learn_point_cost == 0 {
        // TODO: Also ignore clan skills
        return;
    }

    let color = if player.map_or(true, |player| {
        player.skill_points.points >= skill_data.learn_point_cost
    }) {
        egui::Color32::GREEN
    } else {
        egui::Color32::RED
    };

    ui.colored_label(
        color,
        format!(
            "[{}: {}]",
            game_data.client_strings.skill_learn_point_cost, skill_data.learn_point_cost
        ),
    );
}

fn add_skill_require_equipment(
    ui: &mut egui::Ui,
    game_data: &GameData,
    player: Option<&PlayerTooltipQueryItem>,
    skill_data: &SkillData,
) {
    if skill_data.required_ability.is_empty() {
        return;
    }

    let mut text = format!("[{}:", game_data.client_strings.skill_require_ability);
    let mut color = egui::Color32::RED;

    for &item_class in skill_data.required_equipment_class.iter() {
        write!(
            text,
            " {}",
            game_data.string_database.get_item_class(item_class),
        )
        .ok();

        if let Some(player) = player {
            for equipment in player
                .equipment
                .equipped_items
                .iter()
                .filter_map(|(_, x)| x.as_ref())
            {
                if let Some(item_data) = game_data.items.get_base_item(equipment.item) {
                    if item_class == item_data.class {
                        color = egui::Color32::GREEN;
                    }
                }
            }
        }
    }

    text.push(']');
    ui.colored_label(color, text);
}

fn add_skill_requirements(
    ui: &mut egui::Ui,
    game_data: &GameData,
    player: Option<&PlayerTooltipQueryItem>,
    skill_data: &SkillData,
) {
    add_skill_require_job(ui, game_data, player, skill_data);
    add_skill_require_ability(ui, game_data, player, skill_data);
    add_skill_require_skill(ui, game_data, player, skill_data);
    add_skill_require_skill_point(ui, game_data, player, skill_data);
    add_skill_require_equipment(ui, game_data, player, skill_data);
}

fn add_skill_status_effects(
    ui: &mut egui::Ui,
    game_data: &GameData,
    player: Option<&PlayerTooltipQueryItem>,
    skill_data: &SkillData,
) {
    let prefix = if matches!(skill_data.skill_type, SkillType::Passive) {
        game_data.client_strings.skill_passive_ability
    } else {
        game_data.client_strings.skill_status_effects
    };

    let add_skill_add_ability = |text: &mut String, skill_add_ability: &SkillAddAbility| {
        if skill_add_ability.value > 0 {
            let value = if matches!(
                skill_data.skill_type,
                SkillType::SelfBoundDuration
                    | SkillType::TargetBoundDuration
                    | SkillType::SelfBound
                    | SkillType::TargetBound
            ) {
                (skill_add_ability.value as f32
                    * (player.map_or(15.0, |player| {
                        player.ability_values.get_intelligence() as f32
                    }) + 300.0)
                    / 315.0) as i32
            } else {
                skill_add_ability.value
            };

            if matches!(skill_add_ability.ability_type, AbilityType::PassiveSaveMana) {
                write!(text, "{}%", value).ok();
            } else {
                write!(text, "{}", value).ok();
            }
        }

        if skill_add_ability.rate > 0 {
            if skill_add_ability.value > 0 {
                text.push(' ');
            }
            write!(text, "{}%", skill_add_ability.rate).ok();
        }
    };

    for (index, status_effect_id) in skill_data.status_effects.iter().enumerate() {
        if let Some(status_effect_id) = status_effect_id {
            if let Some(status_effect) = game_data
                .status_effects
                .get_status_effect(*status_effect_id)
            {
                let mut text = format!("{}: {}", prefix, status_effect.name);

                if matches!(
                    status_effect.status_effect_type,
                    StatusEffectType::AdditionalDamageRate
                ) {
                    write!(&mut text, " [{}%]", skill_data.power).ok();
                } else if let Some(skill_add_ability) = skill_data.add_ability[index].as_ref() {
                    text.push(' ');
                    text.push('[');
                    add_skill_add_ability(&mut text, skill_add_ability);
                    text.push(']');
                }

                ui.colored_label(egui::Color32::from_rgb(100, 200, 255), text);
            }
        } else if let Some(skill_add_ability) = skill_data.add_ability[index].as_ref() {
            let mut text = format!(
                "{}: {} [",
                prefix,
                game_data
                    .string_database
                    .get_ability_type(skill_add_ability.ability_type)
            );

            add_skill_add_ability(&mut text, skill_add_ability);

            text.push(']');
            ui.colored_label(egui::Color32::from_rgb(100, 200, 255), text);
        }
    }

    if skill_data.status_effects.iter().any(|x| x.is_some()) {
        if skill_data.success_ratio > 0 {
            ui.label(format!(
                "{}: {}-{}% {}: {}{}",
                game_data.client_strings.skill_success_rate,
                (skill_data.success_ratio as f32 * 0.8) as i32,
                skill_data.success_ratio,
                game_data.client_strings.skill_duration,
                skill_data.status_effect_duration.as_secs(),
                game_data.client_strings.duration_seconds,
            ));
        } else {
            ui.label(format!(
                "{}: 100% {}: {} {}",
                game_data.client_strings.skill_success_rate,
                game_data.client_strings.skill_duration,
                skill_data.status_effect_duration.as_secs(),
                game_data.client_strings.duration_seconds,
            ));
        }
    }
}

fn add_skill_steal_ability_value(ui: &mut egui::Ui, game_data: &GameData, skill_data: &SkillData) {
    for skill_add_ability in skill_data.add_ability.iter().filter_map(|x| x.as_ref()) {
        ui.label(format!(
            "{}: {} {}",
            game_data.client_strings.skill_steal_ability,
            game_data
                .string_database
                .get_ability_type(skill_add_ability.ability_type),
            skill_add_ability.value,
        ));
    }
}

fn add_skill_summon_points(ui: &mut egui::Ui, game_data: &GameData, skill_data: &SkillData) {
    if let Some(summon_point_cost) = skill_data
        .summon_npc_id
        .and_then(|id| game_data.npcs.get_npc(id))
        .map(|npc_data| npc_data.summon_point_requirement)
    {
        // TODO: Colour green / red for whether we have enough summon points
        ui.label(format!(
            "{}: {}",
            game_data.client_strings.skill_summon_point_cost, summon_point_cost
        ));
    }
}

fn add_skill_type(ui: &mut egui::Ui, game_data: &GameData, skill_data: &SkillData) {
    ui.label(format!(
        "{}: {}",
        game_data.client_strings.skill_type,
        game_data
            .string_database
            .get_skill_type(skill_data.skill_type)
    ));
}

fn add_skill_target(ui: &mut egui::Ui, game_data: &GameData, skill_data: &SkillData) {
    ui.label(format!(
        "{}: {}",
        game_data.client_strings.skill_target,
        game_data
            .string_database
            .get_skill_target_filter(skill_data.target_filter)
    ));
}

fn add_skill_type_and_target(ui: &mut egui::Ui, game_data: &GameData, skill_data: &SkillData) {
    ui.horizontal(|ui| {
        add_skill_type(ui, game_data, skill_data);
        add_skill_target(ui, game_data, skill_data);
    });
}

fn add_skill_use_ability_value(
    ui: &mut egui::Ui,
    game_data: &GameData,
    player: Option<&PlayerTooltipQueryItem>,
    skill_data: &SkillData,
) {
    for &(ability_type, mut value) in skill_data.use_ability.iter() {
        let mut color = egui::Color32::RED;

        if let Some(player) = player {
            if matches!(ability_type, AbilityType::Mana) {
                let use_mana_rate = (100 - player.ability_values.get_save_mana()) as f32 / 100.0;
                value = (value as f32 * use_mana_rate) as i32;
            }

            if let Some(current_value) = ability_values_get_value(
                ability_type,
                player.ability_values,
                Some(player.character_info),
                Some(player.experience_points),
                Some(player.health_points),
                Some(player.inventory),
                Some(player.level),
                Some(player.mana_points),
                Some(player.move_speed),
                Some(player.skill_points),
                Some(player.stamina),
                Some(player.stat_points),
                Some(player.team),
                Some(player.union_membership),
            ) {
                if current_value >= value as i32 {
                    color = egui::Color32::GREEN;
                }
            }
        }

        ui.colored_label(
            color,
            format!(
                "[{}: {} {}]",
                game_data.client_strings.skill_cost_ability,
                game_data.string_database.get_ability_type(ability_type),
                value
            ),
        );
    }
}

pub enum SkillTooltipType {
    Simple,
    Detailed,
    Extra,
    NextLevel,
}

pub fn ui_add_skill_tooltip(
    ui: &mut egui::Ui,
    tooltip_type: SkillTooltipType,
    game_data: &GameData,
    player: Option<&PlayerTooltipQueryItem>,
    skill_id: SkillId,
) {
    ui.set_max_width(TOOLTIP_MAX_WIDTH);
    ui.style_mut().visuals.widgets.noninteractive.fg_stroke =
        egui::Stroke::new(1.0, egui::Color32::WHITE);

    let skill_data = game_data.skills.get_skill(skill_id);
    if skill_data.is_none() {
        ui.label(format!("Unknown Skill\nSkill ID: {}", skill_id.get()));
        return;
    }
    let skill_data = skill_data.unwrap();

    if matches!(tooltip_type, SkillTooltipType::Simple) {
        add_skill_name(ui, game_data, skill_data);
        add_skill_use_ability_value(ui, game_data, player, skill_data);
    } else {
        match skill_data.skill_type {
            SkillType::BasicAction => {
                if !matches!(tooltip_type, SkillTooltipType::NextLevel) {
                    add_skill_name(ui, game_data, skill_data);
                }

                add_skill_type_and_target(ui, game_data, skill_data);

                add_skill_description(ui, skill_data);
            }
            SkillType::CreateWindow => {
                if !matches!(tooltip_type, SkillTooltipType::NextLevel) {
                    add_skill_name(ui, game_data, skill_data);
                }

                add_skill_type_and_target(ui, game_data, skill_data);

                add_skill_use_ability_value(ui, game_data, player, skill_data);

                add_skill_requirements(ui, game_data, player, skill_data);

                add_skill_description(ui, skill_data);

                if matches!(tooltip_type, SkillTooltipType::Extra) {
                    if let Some(next_level_skill_data) =
                        add_skill_next_level(ui, game_data, skill_data)
                    {
                        ui_add_skill_tooltip(
                            ui,
                            SkillTooltipType::NextLevel,
                            game_data,
                            player,
                            next_level_skill_data.id,
                        );
                    }
                }
            }
            SkillType::Immediate | SkillType::EnforceWeapon | SkillType::EnforceBullet => {
                if !matches!(tooltip_type, SkillTooltipType::NextLevel) {
                    add_skill_name(ui, game_data, skill_data);
                }

                add_skill_type_and_target(ui, game_data, skill_data);
                add_skill_use_ability_value(ui, game_data, player, skill_data);

                add_skill_power(ui, game_data, skill_data);
                add_skill_status_effects(ui, game_data, player, skill_data);

                add_skill_requirements(ui, game_data, player, skill_data);

                add_skill_description(ui, skill_data);

                if matches!(tooltip_type, SkillTooltipType::Extra) {
                    if let Some(next_level_skill_data) =
                        add_skill_next_level(ui, game_data, skill_data)
                    {
                        ui_add_skill_tooltip(
                            ui,
                            SkillTooltipType::NextLevel,
                            game_data,
                            player,
                            next_level_skill_data.id,
                        );
                    }
                }
            }
            SkillType::FireBullet => {
                if !matches!(tooltip_type, SkillTooltipType::NextLevel) {
                    add_skill_name(ui, game_data, skill_data);
                }

                add_skill_type_and_target(ui, game_data, skill_data);
                add_skill_use_ability_value(ui, game_data, player, skill_data);

                add_skill_power(ui, game_data, skill_data);
                add_skill_cast_range(ui, game_data, skill_data);
                add_skill_status_effects(ui, game_data, player, skill_data);

                add_skill_requirements(ui, game_data, player, skill_data);

                add_skill_description(ui, skill_data);

                if matches!(tooltip_type, SkillTooltipType::Extra) {
                    if let Some(next_level_skill_data) =
                        add_skill_next_level(ui, game_data, skill_data)
                    {
                        ui_add_skill_tooltip(
                            ui,
                            SkillTooltipType::NextLevel,
                            game_data,
                            player,
                            next_level_skill_data.id,
                        );
                    }
                }
            }
            SkillType::AreaTarget => {
                if !matches!(tooltip_type, SkillTooltipType::NextLevel) {
                    add_skill_name(ui, game_data, skill_data);
                }

                add_skill_type_and_target(ui, game_data, skill_data);
                add_skill_use_ability_value(ui, game_data, player, skill_data);

                add_skill_power(ui, game_data, skill_data);
                add_skill_cast_range(ui, game_data, skill_data);
                add_skill_aoe_range(ui, game_data, skill_data);
                add_skill_status_effects(ui, game_data, player, skill_data);

                add_skill_requirements(ui, game_data, player, skill_data);

                add_skill_description(ui, skill_data);

                if matches!(tooltip_type, SkillTooltipType::Extra) {
                    if let Some(next_level_skill_data) =
                        add_skill_next_level(ui, game_data, skill_data)
                    {
                        ui_add_skill_tooltip(
                            ui,
                            SkillTooltipType::NextLevel,
                            game_data,
                            player,
                            next_level_skill_data.id,
                        );
                    }
                }
            }
            SkillType::SelfBound | SkillType::SelfBoundDuration | SkillType::SelfStateDuration => {
                if !matches!(tooltip_type, SkillTooltipType::NextLevel) {
                    add_skill_name(ui, game_data, skill_data);
                }

                add_skill_type_and_target(ui, game_data, skill_data);
                add_skill_use_ability_value(ui, game_data, player, skill_data);

                add_skill_aoe_range(ui, game_data, skill_data);
                add_skill_status_effects(ui, game_data, player, skill_data);

                add_skill_requirements(ui, game_data, player, skill_data);

                add_skill_description(ui, skill_data);

                if matches!(tooltip_type, SkillTooltipType::Extra) {
                    if let Some(next_level_skill_data) =
                        add_skill_next_level(ui, game_data, skill_data)
                    {
                        ui_add_skill_tooltip(
                            ui,
                            SkillTooltipType::NextLevel,
                            game_data,
                            player,
                            next_level_skill_data.id,
                        );
                    }
                }
            }
            SkillType::TargetBound
            | SkillType::TargetBoundDuration
            | SkillType::TargetStateDuration => {
                if !matches!(tooltip_type, SkillTooltipType::NextLevel) {
                    add_skill_name(ui, game_data, skill_data);
                }

                add_skill_type_and_target(ui, game_data, skill_data);
                add_skill_use_ability_value(ui, game_data, player, skill_data);

                add_skill_cast_range(ui, game_data, skill_data);
                add_skill_aoe_range(ui, game_data, skill_data);
                add_skill_status_effects(ui, game_data, player, skill_data);

                add_skill_requirements(ui, game_data, player, skill_data);

                add_skill_description(ui, skill_data);

                if matches!(tooltip_type, SkillTooltipType::Extra) {
                    if let Some(next_level_skill_data) =
                        add_skill_next_level(ui, game_data, skill_data)
                    {
                        ui_add_skill_tooltip(
                            ui,
                            SkillTooltipType::NextLevel,
                            game_data,
                            player,
                            next_level_skill_data.id,
                        );
                    }
                }
            }
            SkillType::SummonPet => {
                if !matches!(tooltip_type, SkillTooltipType::NextLevel) {
                    add_skill_name(ui, game_data, skill_data);
                }

                add_skill_type(ui, game_data, skill_data);
                add_skill_use_ability_value(ui, game_data, player, skill_data);

                add_skill_summon_points(ui, game_data, skill_data);

                add_skill_requirements(ui, game_data, player, skill_data);

                add_skill_description(ui, skill_data);

                if matches!(tooltip_type, SkillTooltipType::Extra) {
                    if let Some(next_level_skill_data) =
                        add_skill_next_level(ui, game_data, skill_data)
                    {
                        ui_add_skill_tooltip(
                            ui,
                            SkillTooltipType::NextLevel,
                            game_data,
                            player,
                            next_level_skill_data.id,
                        );
                    }
                }
            }
            SkillType::Passive => {
                if !matches!(tooltip_type, SkillTooltipType::NextLevel) {
                    add_skill_name(ui, game_data, skill_data);
                }

                add_skill_type(ui, game_data, skill_data);
                add_skill_use_ability_value(ui, game_data, player, skill_data);

                add_skill_status_effects(ui, game_data, player, skill_data);

                add_skill_requirements(ui, game_data, player, skill_data);

                add_skill_description(ui, skill_data);

                if matches!(tooltip_type, SkillTooltipType::Extra) {
                    if let Some(next_level_skill_data) =
                        add_skill_next_level(ui, game_data, skill_data)
                    {
                        ui_add_skill_tooltip(
                            ui,
                            SkillTooltipType::NextLevel,
                            game_data,
                            player,
                            next_level_skill_data.id,
                        );
                    }
                }
            }
            SkillType::Emote => {
                if !matches!(tooltip_type, SkillTooltipType::NextLevel) {
                    add_skill_name(ui, game_data, skill_data);
                }

                add_skill_type(ui, game_data, skill_data);
                add_skill_use_ability_value(ui, game_data, player, skill_data);

                add_skill_description(ui, skill_data);

                if matches!(tooltip_type, SkillTooltipType::Extra) {
                    if let Some(next_level_skill_data) =
                        add_skill_next_level(ui, game_data, skill_data)
                    {
                        ui_add_skill_tooltip(
                            ui,
                            SkillTooltipType::NextLevel,
                            game_data,
                            player,
                            next_level_skill_data.id,
                        );
                    }
                }
            }
            SkillType::SelfDamage => {
                if !matches!(tooltip_type, SkillTooltipType::NextLevel) {
                    add_skill_name(ui, game_data, skill_data);
                }

                add_skill_type_and_target(ui, game_data, skill_data);
                add_skill_use_ability_value(ui, game_data, player, skill_data);

                add_skill_power(ui, game_data, skill_data);
                add_skill_aoe_range(ui, game_data, skill_data);
                add_skill_status_effects(ui, game_data, player, skill_data);

                add_skill_requirements(ui, game_data, player, skill_data);

                add_skill_description(ui, skill_data);

                if matches!(tooltip_type, SkillTooltipType::Extra) {
                    if let Some(next_level_skill_data) =
                        add_skill_next_level(ui, game_data, skill_data)
                    {
                        ui_add_skill_tooltip(
                            ui,
                            SkillTooltipType::NextLevel,
                            game_data,
                            player,
                            next_level_skill_data.id,
                        );
                    }
                }
            }
            SkillType::SelfAndTarget => {
                if !matches!(tooltip_type, SkillTooltipType::NextLevel) {
                    add_skill_name(ui, game_data, skill_data);
                }

                add_skill_type_and_target(ui, game_data, skill_data);
                add_skill_use_ability_value(ui, game_data, player, skill_data);

                add_skill_power(ui, game_data, skill_data);
                add_skill_steal_ability_value(ui, game_data, skill_data);
                add_skill_status_effects(ui, game_data, player, skill_data);

                add_skill_requirements(ui, game_data, player, skill_data);

                add_skill_description(ui, skill_data);

                if matches!(tooltip_type, SkillTooltipType::Extra) {
                    if let Some(next_level_skill_data) =
                        add_skill_next_level(ui, game_data, skill_data)
                    {
                        ui_add_skill_tooltip(
                            ui,
                            SkillTooltipType::NextLevel,
                            game_data,
                            player,
                            next_level_skill_data.id,
                        );
                    }
                }
            }
            SkillType::Resurrection => {
                if !matches!(tooltip_type, SkillTooltipType::NextLevel) {
                    add_skill_name(ui, game_data, skill_data);
                }

                add_skill_type_and_target(ui, game_data, skill_data);
                add_skill_use_ability_value(ui, game_data, player, skill_data);

                add_skill_cast_range(ui, game_data, skill_data);
                add_skill_aoe_range(ui, game_data, skill_data);
                add_skill_recover_xp(ui, game_data, skill_data);

                add_skill_requirements(ui, game_data, player, skill_data);

                add_skill_description(ui, skill_data);

                if matches!(tooltip_type, SkillTooltipType::Extra) {
                    if let Some(next_level_skill_data) =
                        add_skill_next_level(ui, game_data, skill_data)
                    {
                        ui_add_skill_tooltip(
                            ui,
                            SkillTooltipType::NextLevel,
                            game_data,
                            player,
                            next_level_skill_data.id,
                        );
                    }
                }
            }
            SkillType::Warp => {
                if !matches!(tooltip_type, SkillTooltipType::NextLevel) {
                    add_skill_name(ui, game_data, skill_data);
                }

                add_skill_type(ui, game_data, skill_data);

                add_skill_description(ui, skill_data);

                if matches!(tooltip_type, SkillTooltipType::Extra) {
                    if let Some(next_level_skill_data) =
                        add_skill_next_level(ui, game_data, skill_data)
                    {
                        ui_add_skill_tooltip(
                            ui,
                            SkillTooltipType::NextLevel,
                            game_data,
                            player,
                            next_level_skill_data.id,
                        );
                    }
                }
            }
        }
    }
}
