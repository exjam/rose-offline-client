use bevy::{
    core::Time,
    hierarchy::BuildChildren,
    prelude::{Commands, Entity, EventReader, Query, Res, ResMut},
};
use bevy_rapier3d::prelude::ColliderShapeComponent;

use rose_game_common::{components::HealthPoints, data::Damage, messages::ClientEntityId};

use crate::{
    components::{ClientEntity, NextCommand, PendingDamageList},
    events::HitEvent,
    resources::{ClientEntityList, DamageDigitsSpawner},
};

// After 3 seconds, apply damage regardless
const MAX_DAMAGE_AGE: f32 = 3.0;

fn apply_damage(
    commands: &mut Commands,
    _attacker_entity: Option<Entity>,
    defender_entity: Entity,
    defender_client_entity_id: ClientEntityId,
    defender_health_points: &mut HealthPoints,
    defender_collider: Option<&ColliderShapeComponent>,
    damage: Damage,
    is_killed: bool,
    damage_digits_spawner: &DamageDigitsSpawner,
    client_entity_list: &mut ClientEntityList,
) {
    if defender_health_points.hp < damage.amount as i32 {
        defender_health_points.hp = 0;
    } else {
        defender_health_points.hp -= damage.amount as i32;
    }

    if let Some(damage_digits_entity) = damage_digits_spawner.spawn(
        commands,
        damage.amount,
        client_entity_list
            .player_entity
            .map_or(false, |player_entity| defender_entity == player_entity),
        defender_collider.map_or(2.0, |collider| collider.compute_local_aabb().extents().y),
    ) {
        commands
            .entity(defender_entity)
            .add_child(damage_digits_entity);
    }

    if is_killed {
        commands
            .entity(defender_entity)
            .insert(NextCommand::with_die())
            .remove::<ClientEntity>();
        client_entity_list.remove(defender_client_entity_id);
    }
}

pub fn pending_damage_system(
    mut commands: Commands,
    mut query_defender: Query<(
        Entity,
        &mut PendingDamageList,
        &ClientEntity,
        &mut HealthPoints,
        Option<&ColliderShapeComponent>,
    )>,
    mut hit_events: EventReader<HitEvent>,
    time: Res<Time>,
    mut client_entity_list: ResMut<ClientEntityList>,
    damage_digits_spawner: Res<DamageDigitsSpawner>,
) {
    let delta_time = time.delta_seconds();

    for event in hit_events.iter() {
        let mut damage = Damage {
            amount: 0,
            is_critical: false,
            apply_hit_stun: false,
        };

        if let Ok((
            _,
            mut pending_damage_list,
            defender_client_entity,
            mut defender_health_points,
            defender_collider,
        )) = query_defender.get_mut(event.defender)
        {
            let mut i = 0;
            let mut is_killed = false;
            while i < pending_damage_list.pending_damage.len() {
                if client_entity_list.get(pending_damage_list.pending_damage[i].attacker)
                    == Some(event.attacker)
                {
                    let pending_damage = pending_damage_list.pending_damage.remove(i);
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
                Some(event.attacker),
                event.defender,
                defender_client_entity.id,
                defender_health_points.as_mut(),
                defender_collider,
                damage,
                is_killed,
                &damage_digits_spawner,
                &mut client_entity_list,
            );
        }
    }

    for (
        defender_entity,
        mut pending_damage_list,
        defender_client_entity,
        mut defender_health_points,
        defender_collider,
    ) in query_defender.iter_mut()
    {
        let mut i = 0;
        while i < pending_damage_list.pending_damage.len() {
            let mut pending_damage = &mut pending_damage_list.pending_damage[i];
            pending_damage.age += delta_time;

            let attacker_entity = client_entity_list.get(pending_damage.attacker);
            if pending_damage.is_immediate
                || pending_damage.age > MAX_DAMAGE_AGE
                || attacker_entity.is_none()
            {
                let pending_damage = pending_damage_list.pending_damage.remove(i);
                apply_damage(
                    &mut commands,
                    attacker_entity,
                    defender_entity,
                    defender_client_entity.id,
                    defender_health_points.as_mut(),
                    defender_collider,
                    pending_damage.damage,
                    pending_damage.is_kill,
                    &damage_digits_spawner,
                    &mut client_entity_list,
                );
            } else {
                i += 1;
            }
        }
    }
}
