use bevy::{
    ecs::prelude::{Query, Res},
    time::Time,
};
use std::time::Duration;

use rose_data::StatusEffectType;
use rose_game_common::components::{
    AbilityValues, ActiveStatusEffectRegen, HealthPoints, ManaPoints, StatusEffects,
    StatusEffectsRegen,
};

use crate::resources::GameData;

fn update_status_effect_regen(regen: &mut ActiveStatusEffectRegen, time: &Time) -> i32 {
    let prev_applied_value = regen.applied_value;

    regen.applied_duration += time.delta();
    regen.applied_value = i32::min(
        ((regen.applied_duration.as_micros() as f32 / 1000000.0) * regen.value_per_second as f32)
            as i32,
        regen.total_value,
    );

    regen.applied_value - prev_applied_value
}

pub fn status_effect_system(
    mut query: Query<(
        &AbilityValues,
        &mut HealthPoints,
        Option<&mut ManaPoints>,
        &StatusEffects,
        &mut StatusEffectsRegen,
    )>,
    game_data: Res<GameData>,
    time: Res<Time>,
) {
    for (
        ability_values,
        mut health_points,
        mut mana_points,
        status_effects,
        mut status_effects_regen,
    ) in query.iter_mut()
    {
        let apply_per_second_effect = {
            status_effects_regen.per_second_tick_counter += time.delta();
            if status_effects_regen.per_second_tick_counter > Duration::from_secs(1) {
                status_effects_regen.per_second_tick_counter -= Duration::from_secs(1);
                true
            } else {
                false
            }
        };

        for (status_effect_type, status_effect_slot) in status_effects.active.iter() {
            if let Some(status_effect) = status_effect_slot {
                match status_effect_type {
                    StatusEffectType::IncreaseHp => {
                        if let Some(status_effect_regen) =
                            &mut status_effects_regen.regens[status_effect_type]
                        {
                            // Calculate regen for this tick
                            let regen = update_status_effect_regen(status_effect_regen, &time);

                            // Update hp
                            let max_hp = ability_values.get_max_health();
                            health_points.hp = i32::min(health_points.hp + regen, max_hp);

                            // Expire when reach max hp
                            if health_points.hp == max_hp {
                                status_effects_regen.regens[status_effect_type] = None;
                            }
                        }
                    }
                    StatusEffectType::IncreaseMp => {
                        if let Some(status_effect_regen) =
                            &mut status_effects_regen.regens[status_effect_type]
                        {
                            if let Some(mana_points) = mana_points.as_mut() {
                                // Calculate regen for this tick
                                let regen = update_status_effect_regen(status_effect_regen, &time);

                                // Update mp
                                let max_mp = ability_values.get_max_mana();
                                mana_points.mp = i32::min(mana_points.mp + regen, max_mp);

                                // Expire when reach max mp
                                if mana_points.mp == max_mp {
                                    status_effects_regen.regens[status_effect_type] = None;
                                }
                            }
                        }
                    }
                    StatusEffectType::Poisoned => {
                        if apply_per_second_effect {
                            if let Some(data) =
                                game_data.status_effects.get_status_effect(status_effect.id)
                            {
                                health_points.hp =
                                    i32::max(health_points.hp - data.apply_per_second_value, 1);
                            }
                        }
                    }
                    StatusEffectType::DecreaseLifeTime => {
                        if apply_per_second_effect {
                            if let Some(data) =
                                game_data.status_effects.get_status_effect(status_effect.id)
                            {
                                if health_points.hp > data.apply_per_second_value {
                                    health_points.hp -= data.apply_per_second_value;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Check if any regen has expired
        for (_, regen_slot) in status_effects_regen.regens.iter_mut() {
            if let Some(regen) = regen_slot.as_ref() {
                if regen.applied_value == regen.total_value {
                    *regen_slot = None;
                }
            }
        }
    }
}
