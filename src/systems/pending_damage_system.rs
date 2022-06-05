use bevy::prelude::{Commands, Entity, Query, Res, ResMut, Time};

use rose_game_common::{components::HealthPoints, data::Damage, messages::ClientEntityId};

use crate::{
    components::{ClientEntity, NextCommand, PendingDamageList},
    resources::ClientEntityList,
};

// After 5 seconds, expire pending damage and apply immediately
const MAX_DAMAGE_AGE: f32 = 5.0;

fn apply_damage(
    commands: &mut Commands,
    defender_entity: Entity,
    defender_client_entity_id: ClientEntityId,
    defender_health_points: &mut HealthPoints,
    damage: Damage,
    is_killed: bool,
    client_entity_list: &mut ClientEntityList,
) {
    if defender_health_points.hp < damage.amount as i32 {
        defender_health_points.hp = 0;
    } else {
        defender_health_points.hp -= damage.amount as i32;
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
    )>,
    time: Res<Time>,
    mut client_entity_list: ResMut<ClientEntityList>,
) {
    let delta_time = time.delta_seconds();

    for (
        defender_entity,
        mut pending_damage_list,
        defender_client_entity,
        mut defender_health_points,
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
                    defender_entity,
                    defender_client_entity.id,
                    defender_health_points.as_mut(),
                    pending_damage.damage,
                    pending_damage.is_kill,
                    &mut client_entity_list,
                );
            } else {
                i += 1;
            }
        }
    }
}
