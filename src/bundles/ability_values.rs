use rose_data::AbilityType;

use rose_game_common::components::{
    AbilityValues, CharacterGender, CharacterInfo, ExperiencePoints, HealthPoints, Inventory,
    Level, ManaPoints, MoveSpeed, SkillPoints, Stamina, StatPoints, Team, UnionMembership,
};

pub fn ability_values_get_value(
    ability_type: AbilityType,
    ability_values: &AbilityValues,
    character_info: Option<&CharacterInfo>,
    experience_points: Option<&ExperiencePoints>,
    health_points: Option<&HealthPoints>,
    inventory: Option<&Inventory>,
    level: Option<&Level>,
    mana_points: Option<&ManaPoints>,
    move_speed: Option<&MoveSpeed>,
    skill_points: Option<&SkillPoints>,
    stamina: Option<&Stamina>,
    stat_points: Option<&StatPoints>,
    team_number: Option<&Team>,
    union_membership: Option<&UnionMembership>,
) -> Option<i32> {
    match ability_type {
        AbilityType::Gender => character_info.map(|x| match x.gender {
            CharacterGender::Male => 0,
            CharacterGender::Female => 1,
        }),
        AbilityType::Race => character_info.map(|x| (x.race / 2) as i32),
        AbilityType::Birthstone => character_info.map(|x| x.birth_stone as i32),
        AbilityType::Class => character_info.map(|x| x.job as i32),
        AbilityType::Rank => character_info.map(|x| x.rank as i32),
        AbilityType::Fame => character_info.map(|x| x.fame as i32),
        AbilityType::FameB => character_info.map(|x| x.fame_b as i32),
        AbilityType::FameG => character_info.map(|x| x.fame_g as i32),
        AbilityType::Face => character_info.map(|x| x.face as i32),
        AbilityType::Hair => character_info.map(|x| x.hair as i32),
        AbilityType::Strength => Some(ability_values.get_strength()),
        AbilityType::Dexterity => Some(ability_values.get_dexterity()),
        AbilityType::Intelligence => Some(ability_values.get_intelligence()),
        AbilityType::Concentration => Some(ability_values.get_concentration()),
        AbilityType::Charm => Some(ability_values.get_charm()),
        AbilityType::Sense => Some(ability_values.get_sense()),
        AbilityType::Attack => Some(ability_values.get_attack_power()),
        AbilityType::Defence => Some(ability_values.get_defence()),
        AbilityType::Hit => Some(ability_values.get_hit()),
        AbilityType::Resistance => Some(ability_values.get_resistance()),
        AbilityType::Avoid => Some(ability_values.get_avoid()),
        AbilityType::AttackSpeed => Some(ability_values.get_attack_speed()),
        AbilityType::Critical => Some(ability_values.get_critical()),
        AbilityType::Speed => move_speed.map(|x| x.speed as i32),
        AbilityType::Skillpoint => skill_points.map(|x| x.points as i32),
        AbilityType::BonusPoint => stat_points.map(|x| x.points as i32),
        AbilityType::Experience => experience_points.map(|x| x.xp as i32),
        AbilityType::Level => level.map(|x| x.level as i32),
        AbilityType::Money => inventory.map(|x| x.money.0 as i32),
        AbilityType::TeamNumber => team_number.map(|x| x.id as i32),
        AbilityType::Union => {
            union_membership.map(|x| x.current_union.map(|x| x.get() as i32).unwrap_or(0))
        }
        AbilityType::UnionPoint1 => union_membership.map(|x| x.points[0] as i32),
        AbilityType::UnionPoint2 => union_membership.map(|x| x.points[1] as i32),
        AbilityType::UnionPoint3 => union_membership.map(|x| x.points[2] as i32),
        AbilityType::UnionPoint4 => union_membership.map(|x| x.points[3] as i32),
        AbilityType::UnionPoint5 => union_membership.map(|x| x.points[4] as i32),
        AbilityType::UnionPoint6 => union_membership.map(|x| x.points[5] as i32),
        AbilityType::UnionPoint7 => union_membership.map(|x| x.points[6] as i32),
        AbilityType::UnionPoint8 => union_membership.map(|x| x.points[7] as i32),
        AbilityType::UnionPoint9 => union_membership.map(|x| x.points[8] as i32),
        AbilityType::UnionPoint10 => union_membership.map(|x| x.points[9] as i32),
        AbilityType::Stamina => stamina.map(|x| x.stamina as i32),
        AbilityType::MaxHealth => Some(ability_values.get_max_health()),
        AbilityType::MaxMana => Some(ability_values.get_max_mana()),
        AbilityType::Health => health_points.map(|x| x.hp),
        AbilityType::Mana => mana_points.map(|x| x.mp),
        /*
        TODO: Implement remaining get ability types.
        AbilityType::Weight => todo!(),
        AbilityType::SaveMana => todo!(),
        AbilityType::PvpFlag => todo!(),
        AbilityType::HeadSize => todo!(),
        AbilityType::BodySize => todo!(),
        AbilityType::DropRate => todo!(),
        AbilityType::CurrentPlanet => todo!(),
        AbilityType::GuildNumber => todo!(),
        AbilityType::GuildScore => todo!(),
        AbilityType::GuildPosition => todo!(),
        */
        _ => {
            log::warn!(
                "ability_values_get_value unimplemented for ability type {:?}",
                ability_type
            );
            None
        }
    }
}
