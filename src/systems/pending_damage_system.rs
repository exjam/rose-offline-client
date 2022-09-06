use bevy::{
    ecs::query::WorldQuery,
    prelude::{Commands, Entity, Query, Res, ResMut, Time},
};

use rose_game_common::{components::HealthPoints, data::Damage};

use crate::{
    components::{ClientEntity, Dead, NextCommand, PendingDamageList},
    resources::ClientEntityList,
};

// After 5 seconds, expire pending damage and apply immediately
const MAX_DAMAGE_AGE: f32 = 5.0;

#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct DamageTarget<'w> {
    entity: Entity,
    client_entity: &'w ClientEntity,
    health_points: &'w mut HealthPoints,
    pending_damage_list: &'w mut PendingDamageList,
}

fn apply_damage(
    commands: &mut Commands,
    target: &mut DamageTargetItem,
    damage: Damage,
    is_killed: bool,
    client_entity_list: &mut ClientEntityList,
) {
    if target.health_points.hp < damage.amount as i32 {
        target.health_points.hp = 0;
    } else {
        target.health_points.hp -= damage.amount as i32;
    }

    if is_killed {
        commands
            .entity(target.entity)
            .insert(Dead)
            .insert(NextCommand::with_die())
            .remove::<ClientEntity>();
        client_entity_list.remove(target.client_entity.id);
    }
}

pub fn pending_damage_system(
    mut commands: Commands,
    mut query_target: Query<DamageTarget>,
    time: Res<Time>,
    mut client_entity_list: ResMut<ClientEntityList>,
) {
    let delta_time = time.delta_seconds();

    for mut target in query_target.iter_mut() {
        let mut i = 0;
        while i < target.pending_damage_list.len() {
            let mut pending_damage = &mut target.pending_damage_list[i];
            pending_damage.age += delta_time;

            let attacker_entity = client_entity_list.get(pending_damage.attacker);
            if pending_damage.is_immediate
                || pending_damage.age > MAX_DAMAGE_AGE
                || attacker_entity.is_none()
            {
                let pending_damage = target.pending_damage_list.remove(i);
                apply_damage(
                    &mut commands,
                    &mut target,
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
