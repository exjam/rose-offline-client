use bevy::prelude::{Changed, Or, QuerySet, QueryState, Res};

use rose_game_common::components::{
    AbilityValues, BasicStats, CharacterInfo, Equipment, HealthPoints, Level, ManaPoints, MoveMode,
    MoveSpeed, Npc, SkillList, StatusEffects,
};

use crate::resources::GameData;

#[allow(clippy::type_complexity)]
pub fn ability_values_system(
    mut query_set: QuerySet<(
        QueryState<
            (
                &mut AbilityValues,
                &CharacterInfo,
                &Level,
                &Equipment,
                &BasicStats,
                &SkillList,
                &StatusEffects,
            ),
            Or<(
                Changed<CharacterInfo>,
                Changed<Level>,
                Changed<Equipment>,
                Changed<BasicStats>,
                Changed<SkillList>,
                Changed<StatusEffects>,
            )>,
        >,
        QueryState<
            (&mut AbilityValues, &Npc, &StatusEffects),
            Or<(Changed<Npc>, Changed<StatusEffects>)>,
        >,
        QueryState<
            (
                &AbilityValues,
                &MoveMode,
                &mut MoveSpeed,
                &mut HealthPoints,
                Option<&mut ManaPoints>,
            ),
            Changed<AbilityValues>,
        >,
    )>,
    game_data: Res<GameData>,
) {
    query_set.q0().for_each_mut(
        |(
            mut ability_values,
            character_info,
            level,
            equipment,
            basic_stats,
            skill_list,
            status_effects,
        )| {
            *ability_values = game_data.ability_value_calculator.calculate(
                character_info,
                level,
                equipment,
                basic_stats,
                skill_list,
                status_effects,
            );
        },
    );

    query_set
        .q1()
        .for_each_mut(|(mut ability_values, npc, status_effects)| {
            *ability_values = game_data
                .ability_value_calculator
                .calculate_npc(
                    npc.id,
                    status_effects,
                    ability_values.summon_owner_level,
                    ability_values.summon_skill_level,
                )
                .unwrap();
        });

    query_set.q2().for_each_mut(
        |(ability_values, move_mode, mut move_speed, mut health_points, mana_points)| {
            // Limit hp to max health
            let max_hp = ability_values.get_max_health();
            if health_points.hp > max_hp {
                health_points.hp = max_hp;
            }

            // Limit mp to max mana
            if let Some(mut mana_points) = mana_points {
                let max_mp = ability_values.get_max_mana();
                if mana_points.mp > max_mp {
                    mana_points.mp = max_mp;
                }
            }

            // Update move speed
            let updated_move_speed = match move_mode {
                MoveMode::Run => ability_values.get_run_speed(),
                MoveMode::Walk => ability_values.get_walk_speed(),
                MoveMode::Drive => ability_values.get_drive_speed(),
            };
            if (move_speed.speed - updated_move_speed).abs() > f32::EPSILON {
                move_speed.speed = updated_move_speed;
            }
        },
    );
}
