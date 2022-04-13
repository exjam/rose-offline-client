use bevy::{
    core::Time,
    prelude::{Entity, EventReader, Mut, Query, Res},
};

use rose_data::{AbilityType, AnimationEventFlags, SkillData, StatusEffectType};
use rose_game_common::components::{
    AbilityValues, HealthPoints, ManaPoints, MoveSpeed, StatusEffects,
};

use crate::{
    bundles::ability_values_get_value,
    components::{PendingSkillEffectList, PendingSkillTargetList},
    events::AnimationFrameEvent,
    resources::GameData,
};

// After 10 seconds, apply skill effects regardless
#[allow(dead_code)]
const MAX_SKILL_EFFECT_AGE: f32 = 10.0;

fn apply_skill_effect(
    skill_data: &SkillData,
    game_data: &GameData,
    time: &Time,
    ability_values: &AbilityValues,
    health_points: &mut Mut<HealthPoints>,
    mana_points: &mut Option<Mut<ManaPoints>>,
    move_speed: &MoveSpeed,
    status_effects: &mut Mut<StatusEffects>,
    caster_intelligence: i32,
    effect_success: [bool; 2],
) {
    for (skill_effect_index, success) in effect_success.iter().enumerate() {
        if !success {
            continue;
        }

        let status_effect_data = skill_data
            .status_effects
            .get(skill_effect_index)
            .and_then(|x| x.as_ref())
            .and_then(|status_effect_id| {
                game_data
                    .status_effects
                    .get_status_effect(*status_effect_id)
            });
        if let Some(status_effect_data) = status_effect_data {
            let adjust_value = if matches!(
                status_effect_data.status_effect_type,
                StatusEffectType::AdditionalDamageRate
            ) {
                skill_data.power as i32
            } else if let Some(skill_add_ability) =
                skill_data.add_ability[skill_effect_index].as_ref()
            {
                // We only need components which can potentially be altered by status effects
                let ability_value = ability_values_get_value(
                    skill_add_ability.ability_type,
                    ability_values,
                    None,
                    None,
                    Some((*health_points).as_ref()),
                    None,
                    None,
                    mana_points.as_ref().map(|x| x.as_ref()),
                    Some(move_speed),
                    None,
                    None,
                    None,
                    None,
                    None,
                )
                .unwrap_or(0);

                game_data
                    .ability_value_calculator
                    .calculate_skill_adjust_value(
                        skill_add_ability,
                        caster_intelligence,
                        ability_value,
                    )
            } else {
                0
            };

            status_effects.apply_status_effect(
                status_effect_data,
                time.last_update().unwrap() + skill_data.status_effect_duration,
                adjust_value,
            );
        }

        let add_ability = skill_data
            .add_ability
            .get(skill_effect_index)
            .and_then(|x| x.as_ref());
        if let Some(add_ability) = add_ability {
            match add_ability.ability_type {
                AbilityType::Health => {
                    health_points.hp = i32::min(
                        ability_values.get_max_health(),
                        health_points.hp
                            + game_data
                                .ability_value_calculator
                                .calculate_skill_adjust_value(
                                    add_ability,
                                    caster_intelligence,
                                    health_points.hp,
                                ),
                    );
                }
                AbilityType::Mana => {
                    if let Some(mana_points) = mana_points.as_mut() {
                        mana_points.mp = i32::min(
                            ability_values.get_max_mana(),
                            mana_points.mp + add_ability.value,
                        );
                    }
                }
                AbilityType::Stamina | AbilityType::Money => {
                    log::warn!(
                        "Unimplemented skill status effect add ability_type {:?}, value {}",
                        add_ability.ability_type,
                        add_ability.value
                    )
                }
                _ => {}
            }
        }
    }
}

pub fn pending_skill_effect_system(
    mut query_caster: Query<(Entity, &mut PendingSkillTargetList)>,
    mut query_defender: Query<(
        Entity,
        &mut PendingSkillEffectList,
        &AbilityValues,
        &mut HealthPoints,
        Option<&mut ManaPoints>,
        &MoveSpeed,
        &mut StatusEffects,
    )>,
    mut animation_frame_events: EventReader<AnimationFrameEvent>,
    game_data: Res<GameData>,
    time: Res<Time>,
) {
    // Apply skill effects triggered by animation frames
    for event in animation_frame_events.iter() {
        if !event
            .flags
            .contains(AnimationEventFlags::APPLY_PENDING_SKILL_EFFECT)
        {
            continue;
        }

        if let Ok((caster_entity, mut caster_pending_skill_effect_list)) =
            query_caster.get_mut(event.entity)
        {
            // Find all our skill targets
            for pending_skill_target in caster_pending_skill_effect_list
                .pending_skill_targets
                .drain(..)
            {
                if let Ok((
                    _,
                    mut defender_pending_skill_effect_list,
                    defender_ability_values,
                    mut defender_health_points,
                    mut defender_mana_points,
                    defender_move_speed,
                    mut defender_status_effects,
                )) = query_defender.get_mut(pending_skill_target.defender_entity)
                {
                    // Apply any skill affects from caster_entity
                    let mut i = 0;
                    while i < defender_pending_skill_effect_list
                        .pending_skill_effects
                        .len()
                    {
                        if defender_pending_skill_effect_list.pending_skill_effects[i].caster_entity
                            != Some(caster_entity)
                        {
                            i += 1;
                            continue;
                        }

                        let pending_skill_effect = defender_pending_skill_effect_list
                            .pending_skill_effects
                            .remove(i);

                        if let Some(skill_data) =
                            game_data.skills.get_skill(pending_skill_effect.skill_id)
                        {
                            apply_skill_effect(
                                skill_data,
                                &game_data,
                                &time,
                                defender_ability_values,
                                &mut defender_health_points,
                                &mut defender_mana_points,
                                defender_move_speed,
                                &mut defender_status_effects,
                                pending_skill_effect.caster_intelligence,
                                pending_skill_effect.effect_success,
                            );
                        }
                    }
                }
            }
        }
    }

    // Apply expired skill effects
    /*
    for (caster_entity, mut pending_skill_effect_list) in query_defender.iter_mut() {
        let mut i = 0;
        while i < pending_skill_effect_list.pending_skill_effects.len() {
            let mut pending_skill_effect = &mut pending_skill_effect_list.pending_skill_effects[i];
            pending_skill_effect.age += delta_time;

            if pending_skill_effect.age > MAX_SKILL_EFFECT_AGE {
                let pending_skill_effect =
                    pending_skill_effect_list.pending_skill_effects.remove(i);

                if let Ok((
                    _,
                    defender_pending_skill_effect_list,
                    defender_health_points,
                    defender_status_effects,
                    defender_mana_points,
                    defender_stamina,
                )) = query_defender.get_mut(pending_skill_target.defender_entity)
                {
                }
            } else {
                i += 1;
            }
        }
    }
    */
}
