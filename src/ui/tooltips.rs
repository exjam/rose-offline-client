use std::cmp::Ordering;
use std::fmt::Write;

use bevy::ecs::query::WorldQuery;
use bevy_egui::egui;

use rose_data::{
    AbilityType, BaseItemData, EquipmentItem, Item, ItemClass, ItemGradeData, ItemType, JobId,
    SkillData, SkillId, SkillType, StackableItem,
};
use rose_game_common::components::{
    AbilityValues, CharacterInfo, ExperiencePoints, HealthPoints, Inventory, Level, ManaPoints,
    MoveSpeed, SkillPoints, Stamina, StatPoints, Team, UnionMembership,
};

use crate::{bundles::ability_values_get_value, resources::GameData};

const TOOLTIP_MAX_WIDTH: f32 = 300.0;

#[derive(WorldQuery)]
pub struct PlayerTooltipQuery<'w> {
    ability_values: &'w AbilityValues,
    character_info: &'w CharacterInfo,
    experience_points: &'w ExperiencePoints,
    health_points: &'w HealthPoints,
    inventory: &'w Inventory,
    level: &'w Level,
    mana_points: &'w ManaPoints,
    move_speed: &'w MoveSpeed,
    skill_points: &'w SkillPoints,
    stamina: &'w Stamina,
    stat_points: &'w StatPoints,
    team: &'w Team,
    union_membership: &'w UnionMembership,
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
            .size(20.0),
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
            .size(20.0),
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
                                "{}: {} {}:{} -{}",
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
                        "{}{}M",
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

fn add_skill_name(ui: &mut egui::Ui, skill_data: &SkillData) {
    let text = if skill_data.name.is_empty() {
        format!("??? [Skill ID: {}]", skill_data.id.get())
    } else if skill_data.level > 1 {
        format!("{} (Level: {})", &skill_data.name, skill_data.level)
    } else {
        skill_data.name.to_string()
    };

    ui.add(egui::Label::new(
        egui::RichText::new(text)
            .color(egui::Color32::YELLOW)
            .size(20.0),
    ));
}

fn add_skill_aoe_range(ui: &mut egui::Ui, skill_data: &SkillData) {
    ui.label(format!("Area: {}m", skill_data.scope / 100));
}

fn add_skill_cast_range(ui: &mut egui::Ui, skill_data: &SkillData) {
    ui.label(format!("Cast Range: {}m", skill_data.cast_range / 100));
}

fn add_skill_description(ui: &mut egui::Ui, skill_data: &SkillData) {
    ui.label(skill_data.description);
}

fn add_skill_power(ui: &mut egui::Ui, skill_data: &SkillData) {
    ui.label(format!("Power: {}", skill_data.power));
}

fn add_skill_recover_xp(ui: &mut egui::Ui, skill_data: &SkillData) {
    ui.label(format!("Recover XP: {}%", skill_data.power));
}

fn add_skill_requirements(_ui: &mut egui::Ui, _skill_data: &SkillData) {
    // TODO: add_skill_require_job
    // TODO: add_skill_require_ability
    // TODO: add_skill_require_skill
    // TODO: add_skill_require_skill_point
    // TODO: add_skill_require_equipment
}

fn add_skill_status_effects(_ui: &mut egui::Ui, _skill_data: &SkillData) {
    // TODO: add_skill_status_effects
}

fn add_skill_steal_ability_value(ui: &mut egui::Ui, skill_data: &SkillData) {
    for skill_add_ability in skill_data.add_ability.iter().filter_map(|x| x.as_ref()) {
        ui.label(format!(
            "Steal: {} {:?}",
            skill_add_ability.value, skill_add_ability.ability_type
        ));
    }
}

fn add_skill_summon_points(_ui: &mut egui::Ui, _skill_data: &SkillData) {
    // TODO: add_skill_summon_points
}

fn add_skill_type(ui: &mut egui::Ui, skill_data: &SkillData) {
    ui.label(format!("Type: {:?}", skill_data.skill_type));
}

fn add_skill_target(ui: &mut egui::Ui, skill_data: &SkillData) {
    ui.label(format!("Target: {:?}", skill_data.target_filter));
}

fn add_skill_use_ability_value(ui: &mut egui::Ui, skill_data: &SkillData) {
    for (ability_type, value) in skill_data.use_ability.iter() {
        // TODO: Colour based on if condition is met
        // TODO: Adjust mana cost for ability_values.save_mana
        ui.label(format!("Cost: {} {:?}", value, ability_type));
    }
}

pub fn ui_add_skill_tooltip(
    ui: &mut egui::Ui,
    summary: bool,
    game_data: &GameData,
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

    if summary {
        add_skill_name(ui, skill_data);
        add_skill_use_ability_value(ui, skill_data);
    } else {
        match skill_data.skill_type {
            SkillType::BasicAction => {
                add_skill_name(ui, skill_data);
                add_skill_type(ui, skill_data);
                add_skill_target(ui, skill_data);
                add_skill_description(ui, skill_data);
            }
            SkillType::CreateWindow => {
                add_skill_name(ui, skill_data);
                add_skill_type(ui, skill_data);
                add_skill_target(ui, skill_data);
                add_skill_use_ability_value(ui, skill_data);

                add_skill_requirements(ui, skill_data);
                add_skill_description(ui, skill_data);
            }
            SkillType::Immediate | SkillType::EnforceWeapon | SkillType::EnforceBullet => {
                add_skill_name(ui, skill_data);
                add_skill_type(ui, skill_data);
                add_skill_target(ui, skill_data);
                add_skill_use_ability_value(ui, skill_data);

                add_skill_power(ui, skill_data);
                add_skill_status_effects(ui, skill_data);

                add_skill_requirements(ui, skill_data);
                add_skill_description(ui, skill_data);
            }
            SkillType::FireBullet => {
                add_skill_name(ui, skill_data);
                add_skill_type(ui, skill_data);
                add_skill_target(ui, skill_data);
                add_skill_use_ability_value(ui, skill_data);

                add_skill_power(ui, skill_data);
                add_skill_cast_range(ui, skill_data);
                add_skill_status_effects(ui, skill_data);

                add_skill_requirements(ui, skill_data);
                add_skill_description(ui, skill_data);
            }
            SkillType::AreaTarget => {
                add_skill_name(ui, skill_data);
                add_skill_type(ui, skill_data);
                add_skill_target(ui, skill_data);
                add_skill_use_ability_value(ui, skill_data);

                add_skill_power(ui, skill_data);
                add_skill_cast_range(ui, skill_data);
                add_skill_aoe_range(ui, skill_data);
                add_skill_status_effects(ui, skill_data);

                add_skill_requirements(ui, skill_data);
                add_skill_description(ui, skill_data);
            }
            SkillType::SelfBound | SkillType::SelfBoundDuration | SkillType::SelfStateDuration => {
                add_skill_name(ui, skill_data);
                add_skill_type(ui, skill_data);
                add_skill_target(ui, skill_data);
                add_skill_use_ability_value(ui, skill_data);

                add_skill_aoe_range(ui, skill_data);
                add_skill_status_effects(ui, skill_data);

                add_skill_requirements(ui, skill_data);
                add_skill_description(ui, skill_data);
            }
            SkillType::TargetBound
            | SkillType::TargetBoundDuration
            | SkillType::TargetStateDuration => {
                add_skill_name(ui, skill_data);
                add_skill_type(ui, skill_data);
                add_skill_target(ui, skill_data);
                add_skill_use_ability_value(ui, skill_data);

                add_skill_cast_range(ui, skill_data);
                add_skill_aoe_range(ui, skill_data);
                add_skill_status_effects(ui, skill_data);

                add_skill_requirements(ui, skill_data);
                add_skill_description(ui, skill_data);
            }
            SkillType::SummonPet => {
                add_skill_name(ui, skill_data);
                add_skill_type(ui, skill_data);
                add_skill_use_ability_value(ui, skill_data);

                add_skill_summon_points(ui, skill_data);

                add_skill_requirements(ui, skill_data);
                add_skill_description(ui, skill_data);
            }
            SkillType::Passive => {
                add_skill_name(ui, skill_data);
                add_skill_type(ui, skill_data);
                add_skill_use_ability_value(ui, skill_data);

                add_skill_status_effects(ui, skill_data);

                add_skill_requirements(ui, skill_data);
                add_skill_description(ui, skill_data);
            }
            SkillType::Emote => {
                add_skill_name(ui, skill_data);
                add_skill_type(ui, skill_data);
                add_skill_use_ability_value(ui, skill_data);

                add_skill_description(ui, skill_data);
            }
            SkillType::SelfDamage => {
                add_skill_name(ui, skill_data);
                add_skill_type(ui, skill_data);
                add_skill_target(ui, skill_data);
                add_skill_use_ability_value(ui, skill_data);

                add_skill_power(ui, skill_data);
                add_skill_aoe_range(ui, skill_data);
                add_skill_status_effects(ui, skill_data);

                add_skill_requirements(ui, skill_data);
                add_skill_description(ui, skill_data);
            }
            SkillType::SelfAndTarget => {
                add_skill_name(ui, skill_data);
                add_skill_type(ui, skill_data);
                add_skill_target(ui, skill_data);
                add_skill_use_ability_value(ui, skill_data);

                add_skill_power(ui, skill_data);
                add_skill_steal_ability_value(ui, skill_data);
                add_skill_status_effects(ui, skill_data);

                add_skill_requirements(ui, skill_data);
                add_skill_description(ui, skill_data);
            }
            SkillType::Resurrection => {
                add_skill_name(ui, skill_data);
                add_skill_type(ui, skill_data);
                add_skill_target(ui, skill_data);
                add_skill_use_ability_value(ui, skill_data);

                add_skill_cast_range(ui, skill_data);
                add_skill_aoe_range(ui, skill_data);
                add_skill_recover_xp(ui, skill_data);

                add_skill_requirements(ui, skill_data);
                add_skill_description(ui, skill_data);
            }
            SkillType::Warp => {
                add_skill_name(ui, skill_data);
                add_skill_type(ui, skill_data);
                add_skill_description(ui, skill_data);
            }
        }
    }
}
