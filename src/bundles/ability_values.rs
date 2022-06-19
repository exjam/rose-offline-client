use std::num::NonZeroUsize;

use bevy::{ecs::world::EntityMut, prelude::Mut};
use num_traits::{AsPrimitive, Saturating, Signed};
use rose_data::AbilityType;

use rose_game_common::components::{
    AbilityValues, BasicStats, CharacterGender, CharacterInfo, ExperiencePoints, HealthPoints,
    Inventory, Level, ManaPoints, Money, MoveSpeed, SkillPoints, Stamina, StatPoints, Team,
    UnionMembership, MAX_STAMINA,
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
    team: Option<&Team>,
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
        AbilityType::TeamNumber => team.map(|x| x.id as i32),
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

fn add_value<T: Saturating + Copy + 'static, U: Signed + AsPrimitive<T>>(
    value: T,
    add_value: U,
) -> T {
    if add_value.is_negative() {
        value.saturating_sub(add_value.abs().as_())
    } else {
        value.saturating_add(add_value.as_())
    }
}

pub fn ability_values_add_value(
    ability_type: AbilityType,
    value: i32,
    ability_values: &AbilityValues,
    basic_stats: &mut Mut<BasicStats>,
    experience_points: &mut Mut<ExperiencePoints>,
    health_points: &mut Mut<HealthPoints>,
    inventory: &mut Mut<Inventory>,
    level: &mut Mut<Level>,
    mana_points: &mut Mut<ManaPoints>,
    skill_points: &mut Mut<SkillPoints>,
    stamina: &mut Mut<Stamina>,
    stat_points: &mut Mut<StatPoints>,
    union_membership: &mut Mut<UnionMembership>,
) -> bool {
    match ability_type {
        AbilityType::Strength => {
            basic_stats.strength = add_value(basic_stats.strength, value);
        }
        AbilityType::Dexterity => {
            basic_stats.dexterity = add_value(basic_stats.dexterity, value);
        }
        AbilityType::Intelligence => {
            basic_stats.intelligence = add_value(basic_stats.intelligence, value);
        }
        AbilityType::Concentration => {
            basic_stats.concentration = add_value(basic_stats.concentration, value);
        }
        AbilityType::Charm => {
            basic_stats.charm = add_value(basic_stats.charm, value);
        }
        AbilityType::Sense => {
            basic_stats.sense = add_value(basic_stats.sense, value);
        }
        AbilityType::BonusPoint => {
            stat_points.points = add_value(stat_points.points, value);
        }
        AbilityType::Skillpoint => {
            skill_points.points = add_value(skill_points.points, value);
        }
        AbilityType::Money => {
            inventory.try_add_money(Money(value as i64)).ok();
        }
        AbilityType::UnionPoint1 => {
            union_membership.points[0] = add_value(union_membership.points[0], value);
        }
        AbilityType::UnionPoint2 => {
            union_membership.points[1] = add_value(union_membership.points[1], value);
        }
        AbilityType::UnionPoint3 => {
            union_membership.points[2] = add_value(union_membership.points[2], value);
        }
        AbilityType::UnionPoint4 => {
            union_membership.points[3] = add_value(union_membership.points[3], value);
        }
        AbilityType::UnionPoint5 => {
            union_membership.points[4] = add_value(union_membership.points[4], value);
        }
        AbilityType::UnionPoint6 => {
            union_membership.points[5] = add_value(union_membership.points[5], value);
        }
        AbilityType::UnionPoint7 => {
            union_membership.points[6] = add_value(union_membership.points[6], value);
        }
        AbilityType::UnionPoint8 => {
            union_membership.points[7] = add_value(union_membership.points[7], value);
        }
        AbilityType::UnionPoint9 => {
            union_membership.points[8] = add_value(union_membership.points[8], value);
        }
        AbilityType::UnionPoint10 => {
            union_membership.points[9] = add_value(union_membership.points[9], value);
        }
        AbilityType::Stamina => {
            stamina.stamina = u32::min(add_value(stamina.stamina, value), MAX_STAMINA);
        }
        AbilityType::Health => {
            health_points.hp = i32::min(
                add_value(health_points.hp, value),
                ability_values.get_max_health(),
            );
        }
        AbilityType::Mana => {
            mana_points.mp = i32::min(
                add_value(mana_points.mp, value),
                ability_values.get_max_mana(),
            );
        }
        AbilityType::Experience => {
            experience_points.xp = add_value(experience_points.xp, value);
        }
        AbilityType::Level => {
            level.level = add_value(level.level, value);
        }
        _ => {
            log::warn!(
                "ability_values_add_value unimplemented for ability type {:?}",
                ability_type
            );
            return false;
        }
    }

    true
}

pub fn ability_values_add_value_exclusive(
    ability_type: AbilityType,
    value: i32,
    entity: &mut EntityMut,
) -> bool {
    match ability_type {
        AbilityType::Strength => {
            if let Some(mut basic_stats) = entity.get_mut::<BasicStats>() {
                basic_stats.strength = add_value(basic_stats.strength, value);
            }
        }
        AbilityType::Dexterity => {
            if let Some(mut basic_stats) = entity.get_mut::<BasicStats>() {
                basic_stats.dexterity = add_value(basic_stats.dexterity, value);
            }
        }
        AbilityType::Intelligence => {
            if let Some(mut basic_stats) = entity.get_mut::<BasicStats>() {
                basic_stats.intelligence = add_value(basic_stats.intelligence, value);
            }
        }
        AbilityType::Concentration => {
            if let Some(mut basic_stats) = entity.get_mut::<BasicStats>() {
                basic_stats.concentration = add_value(basic_stats.concentration, value);
            }
        }
        AbilityType::Charm => {
            if let Some(mut basic_stats) = entity.get_mut::<BasicStats>() {
                basic_stats.charm = add_value(basic_stats.charm, value);
            }
        }
        AbilityType::Sense => {
            if let Some(mut basic_stats) = entity.get_mut::<BasicStats>() {
                basic_stats.sense = add_value(basic_stats.sense, value);
            }
        }
        AbilityType::BonusPoint => {
            if let Some(mut stat_points) = entity.get_mut::<StatPoints>() {
                stat_points.points = add_value(stat_points.points, value);
            }
        }
        AbilityType::Skillpoint => {
            if let Some(mut skill_points) = entity.get_mut::<SkillPoints>() {
                skill_points.points = add_value(skill_points.points, value);
            }
        }
        AbilityType::Money => {
            if let Some(mut inventory) = entity.get_mut::<Inventory>() {
                inventory.try_add_money(Money(value as i64)).ok();
            }
        }
        AbilityType::UnionPoint1 => {
            if let Some(mut union_membership) = entity.get_mut::<UnionMembership>() {
                union_membership.points[0] = add_value(union_membership.points[0], value);
            }
        }
        AbilityType::UnionPoint2 => {
            if let Some(mut union_membership) = entity.get_mut::<UnionMembership>() {
                union_membership.points[1] = add_value(union_membership.points[1], value);
            }
        }
        AbilityType::UnionPoint3 => {
            if let Some(mut union_membership) = entity.get_mut::<UnionMembership>() {
                union_membership.points[2] = add_value(union_membership.points[2], value);
            }
        }
        AbilityType::UnionPoint4 => {
            if let Some(mut union_membership) = entity.get_mut::<UnionMembership>() {
                union_membership.points[3] = add_value(union_membership.points[3], value);
            }
        }
        AbilityType::UnionPoint5 => {
            if let Some(mut union_membership) = entity.get_mut::<UnionMembership>() {
                union_membership.points[4] = add_value(union_membership.points[4], value);
            }
        }
        AbilityType::UnionPoint6 => {
            if let Some(mut union_membership) = entity.get_mut::<UnionMembership>() {
                union_membership.points[5] = add_value(union_membership.points[5], value);
            }
        }
        AbilityType::UnionPoint7 => {
            if let Some(mut union_membership) = entity.get_mut::<UnionMembership>() {
                union_membership.points[6] = add_value(union_membership.points[6], value);
            }
        }
        AbilityType::UnionPoint8 => {
            if let Some(mut union_membership) = entity.get_mut::<UnionMembership>() {
                union_membership.points[7] = add_value(union_membership.points[7], value);
            }
        }
        AbilityType::UnionPoint9 => {
            if let Some(mut union_membership) = entity.get_mut::<UnionMembership>() {
                union_membership.points[8] = add_value(union_membership.points[8], value);
            }
        }
        AbilityType::UnionPoint10 => {
            if let Some(mut union_membership) = entity.get_mut::<UnionMembership>() {
                union_membership.points[9] = add_value(union_membership.points[9], value);
            }
        }
        AbilityType::Stamina => {
            if let Some(mut stamina) = entity.get_mut::<Stamina>() {
                stamina.stamina = u32::min(add_value(stamina.stamina, value), MAX_STAMINA);
            }
        }
        AbilityType::Health => {
            let max_hp = entity
                .get::<AbilityValues>()
                .map(|ability_values| ability_values.get_max_health());

            if let Some(mut health_points) = entity.get_mut::<HealthPoints>() {
                let mut new_hp = add_value(health_points.hp, value);
                if let Some(max_hp) = max_hp {
                    new_hp = new_hp.min(max_hp);
                }

                health_points.hp = new_hp;
            }
        }
        AbilityType::Mana => {
            let max_mp = entity
                .get::<AbilityValues>()
                .map(|ability_values| ability_values.get_max_mana());

            if let Some(mut mana_points) = entity.get_mut::<ManaPoints>() {
                let mut new_mp = add_value(mana_points.mp, value);
                if let Some(max_mp) = max_mp {
                    new_mp = new_mp.min(max_mp);
                }

                mana_points.mp = new_mp;
            }
        }
        AbilityType::Experience => {
            if let Some(mut experience_points) = entity.get_mut::<ExperiencePoints>() {
                experience_points.xp = add_value(experience_points.xp, value);
            }
        }
        AbilityType::Level => {
            if let Some(mut level) = entity.get_mut::<Level>() {
                level.level = add_value(level.level, value);
            }
        }
        _ => {
            log::warn!(
                "ability_values_add_value unimplemented for ability type {:?}",
                ability_type
            );
            return false;
        }
    }

    true
}

pub fn ability_values_set_value(
    ability_type: AbilityType,
    value: i32,
    ability_values: &AbilityValues,
    basic_stats: &mut Mut<BasicStats>,
    character_info: &mut Mut<CharacterInfo>,
    health_points: &mut Mut<HealthPoints>,
    mana_points: &mut Mut<ManaPoints>,
    experience_points: &mut Mut<ExperiencePoints>,
    level: &mut Mut<Level>,
    team: &mut Mut<Team>,
    union_membership: &mut Mut<UnionMembership>,
) -> bool {
    match ability_type {
        AbilityType::Gender => {
            if value == 0 {
                character_info.gender = CharacterGender::Male;
            } else {
                character_info.gender = CharacterGender::Female;
            }
        }
        AbilityType::Face => {
            character_info.face = value as u8;
        }
        AbilityType::Hair => {
            character_info.hair = value as u8;
        }
        AbilityType::Class => {
            character_info.job = value as u16;
        }
        AbilityType::Strength => {
            basic_stats.strength = value;
        }
        AbilityType::Dexterity => {
            basic_stats.dexterity = value;
        }
        AbilityType::Intelligence => {
            basic_stats.intelligence = value;
        }
        AbilityType::Concentration => {
            basic_stats.concentration = value;
        }
        AbilityType::Charm => {
            basic_stats.charm = value;
        }
        AbilityType::Sense => {
            basic_stats.sense = value;
        }
        AbilityType::Union => {
            if value == 0 {
                union_membership.current_union = None;
            } else {
                union_membership.current_union = NonZeroUsize::new(value as usize);
            }
        }
        AbilityType::UnionPoint1 => {
            union_membership.points[0] = value as u32;
        }
        AbilityType::UnionPoint2 => {
            union_membership.points[1] = value as u32;
        }
        AbilityType::UnionPoint3 => {
            union_membership.points[2] = value as u32;
        }
        AbilityType::UnionPoint4 => {
            union_membership.points[3] = value as u32;
        }
        AbilityType::UnionPoint5 => {
            union_membership.points[4] = value as u32;
        }
        AbilityType::UnionPoint6 => {
            union_membership.points[5] = value as u32;
        }
        AbilityType::UnionPoint7 => {
            union_membership.points[6] = value as u32;
        }
        AbilityType::UnionPoint8 => {
            union_membership.points[7] = value as u32;
        }
        AbilityType::UnionPoint9 => {
            union_membership.points[8] = value as u32;
        }
        AbilityType::UnionPoint10 => {
            union_membership.points[9] = value as u32;
        }
        AbilityType::Health => {
            health_points.hp = i32::min(value, ability_values.get_max_health());
        }
        AbilityType::Mana => {
            mana_points.mp = i32::min(value, ability_values.get_max_mana());
        }
        AbilityType::Experience => experience_points.xp = value as u64,
        AbilityType::Level => level.level = value as u32,
        AbilityType::TeamNumber => team.id = value as u32,
        /*
        TODO: Implement remaining set ability types
        AbilityType::PvpFlag => false,
        */
        _ => {
            log::warn!(
                "ability_values_set_value unimplemented for ability type {:?}",
                ability_type
            );
            return false;
        }
    }

    true
}

pub fn ability_values_set_value_exclusive(
    ability_type: AbilityType,
    value: i32,
    entity: &mut EntityMut,
) -> bool {
    match ability_type {
        AbilityType::Gender => {
            if let Some(mut character_info) = entity.get_mut::<CharacterInfo>() {
                if value == 0 {
                    character_info.gender = CharacterGender::Male;
                } else {
                    character_info.gender = CharacterGender::Female;
                }
            }
        }
        AbilityType::Face => {
            if let Some(mut character_info) = entity.get_mut::<CharacterInfo>() {
                character_info.face = value as u8;
            }
        }
        AbilityType::Hair => {
            if let Some(mut character_info) = entity.get_mut::<CharacterInfo>() {
                character_info.hair = value as u8;
            }
        }
        AbilityType::Class => {
            if let Some(mut character_info) = entity.get_mut::<CharacterInfo>() {
                character_info.job = value as u16;
            }
        }
        AbilityType::Strength => {
            if let Some(mut basic_stats) = entity.get_mut::<BasicStats>() {
                basic_stats.strength = value;
            }
        }
        AbilityType::Dexterity => {
            if let Some(mut basic_stats) = entity.get_mut::<BasicStats>() {
                basic_stats.dexterity = value;
            }
        }
        AbilityType::Intelligence => {
            if let Some(mut basic_stats) = entity.get_mut::<BasicStats>() {
                basic_stats.intelligence = value;
            }
        }
        AbilityType::Concentration => {
            if let Some(mut basic_stats) = entity.get_mut::<BasicStats>() {
                basic_stats.concentration = value;
            }
        }
        AbilityType::Charm => {
            if let Some(mut basic_stats) = entity.get_mut::<BasicStats>() {
                basic_stats.charm = value;
            }
        }
        AbilityType::Sense => {
            if let Some(mut basic_stats) = entity.get_mut::<BasicStats>() {
                basic_stats.sense = value;
            }
        }
        AbilityType::Union => {
            if let Some(mut union_membership) = entity.get_mut::<UnionMembership>() {
                if value == 0 {
                    union_membership.current_union = None;
                } else {
                    union_membership.current_union = NonZeroUsize::new(value as usize);
                }
            }
        }
        AbilityType::UnionPoint1 => {
            if let Some(mut union_membership) = entity.get_mut::<UnionMembership>() {
                union_membership.points[0] = value as u32;
            }
        }
        AbilityType::UnionPoint2 => {
            if let Some(mut union_membership) = entity.get_mut::<UnionMembership>() {
                union_membership.points[1] = value as u32;
            }
        }
        AbilityType::UnionPoint3 => {
            if let Some(mut union_membership) = entity.get_mut::<UnionMembership>() {
                union_membership.points[2] = value as u32;
            }
        }
        AbilityType::UnionPoint4 => {
            if let Some(mut union_membership) = entity.get_mut::<UnionMembership>() {
                union_membership.points[3] = value as u32;
            }
        }
        AbilityType::UnionPoint5 => {
            if let Some(mut union_membership) = entity.get_mut::<UnionMembership>() {
                union_membership.points[4] = value as u32;
            }
        }
        AbilityType::UnionPoint6 => {
            if let Some(mut union_membership) = entity.get_mut::<UnionMembership>() {
                union_membership.points[5] = value as u32;
            }
        }
        AbilityType::UnionPoint7 => {
            if let Some(mut union_membership) = entity.get_mut::<UnionMembership>() {
                union_membership.points[6] = value as u32;
            }
        }
        AbilityType::UnionPoint8 => {
            if let Some(mut union_membership) = entity.get_mut::<UnionMembership>() {
                union_membership.points[7] = value as u32;
            }
        }
        AbilityType::UnionPoint9 => {
            if let Some(mut union_membership) = entity.get_mut::<UnionMembership>() {
                union_membership.points[8] = value as u32;
            }
        }
        AbilityType::UnionPoint10 => {
            if let Some(mut union_membership) = entity.get_mut::<UnionMembership>() {
                union_membership.points[9] = value as u32;
            }
        }
        AbilityType::Health => {
            let max_hp = entity
                .get::<AbilityValues>()
                .map(|ability_values| ability_values.get_max_health());

            if let Some(mut health_points) = entity.get_mut::<HealthPoints>() {
                let mut new_hp = value;
                if let Some(max_hp) = max_hp {
                    new_hp = new_hp.min(max_hp);
                }

                health_points.hp = new_hp;
            }
        }
        AbilityType::Mana => {
            let max_mp = entity
                .get::<AbilityValues>()
                .map(|ability_values| ability_values.get_max_mana());

            if let Some(mut mana_points) = entity.get_mut::<ManaPoints>() {
                let mut new_mp = value;
                if let Some(max_mp) = max_mp {
                    new_mp = new_mp.min(max_mp);
                }

                mana_points.mp = new_mp;
            }
        }
        AbilityType::Experience => {
            if let Some(mut experience_points) = entity.get_mut::<ExperiencePoints>() {
                experience_points.xp = value as u64;
            }
        }
        AbilityType::Level => {
            if let Some(mut level) = entity.get_mut::<Level>() {
                level.level = value as u32;
            }
        }
        AbilityType::TeamNumber => {
            if let Some(mut team) = entity.get_mut::<Team>() {
                team.id = value as u32;
            }
        }
        /*
        TODO: Implement remaining set ability types
        AbilityType::PvpFlag => false,
        */
        _ => {
            log::warn!(
                "ability_values_set_value unimplemented for ability type {:?}",
                ability_type
            );
            return false;
        }
    }

    true
}
