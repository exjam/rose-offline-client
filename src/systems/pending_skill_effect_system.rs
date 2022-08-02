use bevy::{
    ecs::query::WorldQuery,
    prelude::{Entity, EventReader, EventWriter, Query, Res, Time},
};

use rose_data::{AbilityType, AnimationEventFlags, SkillData, StatusEffectType};
use rose_game_common::components::{
    AbilityValues, HealthPoints, ManaPoints, MoveSpeed, StatusEffects,
};

use crate::{
    bundles::ability_values_get_value,
    components::{PendingSkillEffectList, PendingSkillTargetList},
    events::{AnimationFrameEvent, HitEvent},
    resources::GameData,
};

// After 10 seconds, apply skill effects regardless
#[allow(dead_code)]
const MAX_SKILL_EFFECT_AGE: f32 = 10.0;

#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct SkillEffectTarget<'w> {
    entity: Entity,
    ability_values: &'w AbilityValues,
    health_points: &'w mut HealthPoints,
    mana_points: Option<&'w mut ManaPoints>,
    move_speed: &'w MoveSpeed,
    pending_skill_effect_list: &'w mut PendingSkillEffectList,
    status_effects: &'w mut StatusEffects,
}

fn apply_skill_effect(
    skill_data: &SkillData,
    game_data: &GameData,
    time: &Time,
    target: &mut SkillEffectTargetItem,
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
                    target.ability_values,
                    None,
                    None,
                    Some(&target.health_points),
                    None,
                    None,
                    target.mana_points.as_ref().map(|x| x.as_ref()),
                    Some(target.move_speed),
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

            target.status_effects.apply_status_effect(
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
                    target.health_points.hp = i32::min(
                        target.ability_values.get_max_health(),
                        target.health_points.hp
                            + game_data
                                .ability_value_calculator
                                .calculate_skill_adjust_value(
                                    add_ability,
                                    caster_intelligence,
                                    target.health_points.hp,
                                ),
                    );
                }
                AbilityType::Mana => {
                    if let Some(mana_points) = target.mana_points.as_mut() {
                        mana_points.mp = i32::min(
                            target.ability_values.get_max_mana(),
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
    mut query_target: Query<SkillEffectTarget>,
    mut animation_frame_events: EventReader<AnimationFrameEvent>,
    mut hit_events: EventWriter<HitEvent>,
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

        if let Ok((caster_entity, mut caster_pending_skill_target_list)) =
            query_caster.get_mut(event.entity)
        {
            // Find all our skill targets
            for pending_skill_target in caster_pending_skill_target_list.drain(..) {
                if let Ok(mut target) = query_target.get_mut(pending_skill_target.defender_entity) {
                    // Apply any skill affects from caster_entity
                    let mut i = 0;
                    while i < target.pending_skill_effect_list.len() {
                        if target.pending_skill_effect_list[i].caster_entity != Some(caster_entity)
                        {
                            i += 1;
                            continue;
                        }

                        let pending_skill_effect = target
                            .pending_skill_effect_list
                            .pending_skill_effects
                            .remove(i);

                        if let Some(skill_data) =
                            game_data.skills.get_skill(pending_skill_effect.skill_id)
                        {
                            hit_events.send(HitEvent::with_skill(
                                event.entity,
                                target.entity,
                                pending_skill_effect.skill_id,
                            ));

                            apply_skill_effect(
                                skill_data,
                                &game_data,
                                &time,
                                &mut target,
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
        while i < pending_skill_effect_list.len() {
            let mut pending_skill_effect = &mut pending_skill_effect_list[i];
            pending_skill_effect.age += delta_time;

            if pending_skill_effect.age > MAX_SKILL_EFFECT_AGE {
                let pending_skill_effect =
                    pending_skill_effect_list.remove(i);

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
