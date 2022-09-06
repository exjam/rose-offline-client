use bevy::{
    ecs::query::WorldQuery,
    prelude::{Commands, Entity, EventReader, EventWriter, GlobalTransform, Query, Res, ResMut},
};

use rose_game_common::{
    components::{AbilityValues, HealthPoints, ManaPoints, MoveSpeed, StatusEffects},
    data::Damage,
};

use crate::{
    components::{
        ClientEntity, Dead, ModelHeight, NextCommand, PendingDamageList, PendingSkillEffectList,
        PendingSkillTargetList,
    },
    events::{HitEvent, SpawnEffectData, SpawnEffectEvent},
    resources::{ClientEntityList, DamageDigitsSpawner, GameData},
};

#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct HitAttackerQuery<'w> {
    entity: Entity,
    pending_skill_target_list: &'w mut PendingSkillTargetList,
}

#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct HitDefenderQuery<'w> {
    entity: Entity,
    client_entity: &'w ClientEntity,
    pending_damage_list: &'w mut PendingDamageList,
    pending_skill_effect_list: &'w mut PendingSkillEffectList,
    ability_values: &'w AbilityValues,
    health_points: &'w mut HealthPoints,
    global_transform: &'w GlobalTransform,
    mana_points: Option<&'w mut ManaPoints>,
    model_height: Option<&'w ModelHeight>,
    move_speed: &'w MoveSpeed,
    status_effects: &'w mut StatusEffects,
}

fn apply_damage(
    commands: &mut Commands,
    defender: &mut HitDefenderQueryItem,
    damage: Damage,
    is_killed: bool,
    damage_digits_spawner: &DamageDigitsSpawner,
    client_entity_list: &mut ClientEntityList,
) {
    if defender.health_points.hp < damage.amount as i32 {
        defender.health_points.hp = 0;
    } else {
        defender.health_points.hp -= damage.amount as i32;
    }

    damage_digits_spawner.spawn(
        commands,
        defender.global_transform,
        defender
            .model_height
            .map_or(1.8, |model_height| model_height.height),
        damage.amount,
        client_entity_list
            .player_entity
            .map_or(false, |player_entity| defender.entity == player_entity),
    );

    if is_killed {
        commands
            .entity(defender.entity)
            .insert(Dead)
            .insert(NextCommand::with_die())
            .remove::<ClientEntity>();
        client_entity_list.remove(defender.client_entity.id);
    }
}

pub fn hit_event_system(
    mut commands: Commands,
    mut query_defender: Query<HitDefenderQuery>,
    mut hit_events: EventReader<HitEvent>,
    mut spawn_effect_events: EventWriter<SpawnEffectEvent>,
    mut client_entity_list: ResMut<ClientEntityList>,
    damage_digits_spawner: Res<DamageDigitsSpawner>,
    game_data: Res<GameData>,
) {
    for event in hit_events.iter() {
        let defender = query_defender.get_mut(event.defender).ok();
        if defender.is_none() {
            continue;
        }
        let mut defender = defender.unwrap();

        // Apply pending damage
        let mut damage = Damage {
            amount: 0,
            is_critical: false,
            apply_hit_stun: false,
        };

        if event.apply_damage {
            let mut i = 0;
            let mut is_killed = false;
            while i < defender.pending_damage_list.len() {
                if client_entity_list.get(defender.pending_damage_list[i].attacker)
                    == Some(event.attacker)
                    && event.skill_id
                        == defender.pending_damage_list[i]
                            .from_skill
                            .map(|(damage_skill_id, _)| damage_skill_id)
                {
                    let pending_damage = defender.pending_damage_list.remove(i);
                    damage.amount += pending_damage.damage.amount;
                    damage.is_critical |= pending_damage.damage.is_critical;
                    damage.apply_hit_stun |= pending_damage.damage.apply_hit_stun;
                    is_killed |= pending_damage.is_kill;
                } else {
                    i += 1;
                }
            }

            apply_damage(
                &mut commands,
                &mut defender,
                damage,
                is_killed,
                &damage_digits_spawner,
                &mut client_entity_list,
            );
        }

        if let Some(effect_data) = event
            .effect_id
            .and_then(|id| game_data.effect_database.get_effect(id))
        {
            if damage.is_critical {
                if let Some(effect_file_id) = effect_data.hit_effect_critical {
                    spawn_effect_events.send(SpawnEffectEvent::AtEntity(
                        defender.entity,
                        SpawnEffectData::with_file_id(effect_file_id),
                    ));
                }
            }

            if let Some(effect_file_id) = effect_data.hit_effect_normal {
                spawn_effect_events.send(SpawnEffectEvent::AtEntity(
                    defender.entity,
                    SpawnEffectData::with_file_id(effect_file_id),
                ));
            }
        }

        if let Some(skill_data) = event.skill_id.and_then(|id| game_data.skills.get_skill(id)) {
            if let Some(effect_file_id) = skill_data.hit_effect_file_id {
                spawn_effect_events.send(SpawnEffectEvent::OnEntity(
                    defender.entity,
                    skill_data.hit_link_dummy_bone_id,
                    SpawnEffectData::with_file_id(effect_file_id),
                ));
            }
        }
    }
}
