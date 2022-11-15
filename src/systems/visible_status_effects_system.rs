use bevy::{
    hierarchy::{BuildChildren, DespawnRecursiveExt},
    prelude::{
        Changed, Commands, ComputedVisibility, Entity, EventWriter, GlobalTransform, Query, Res,
        Transform, Visibility,
    },
};
use rose_game_common::components::StatusEffects;

use crate::{
    components::{VisibleStatusEffect, VisibleStatusEffects},
    events::{SpawnEffectData, SpawnEffectEvent},
    resources::GameData,
};

pub fn visible_status_effects_system(
    mut commands: Commands,
    mut query_status_effects: Query<
        (Entity, &StatusEffects, &mut VisibleStatusEffects),
        Changed<StatusEffects>,
    >,
    mut spawn_effect_events: EventWriter<SpawnEffectEvent>,
    game_data: Res<GameData>,
) {
    for (entity, status_effects, mut visible_status_effects) in query_status_effects.iter_mut() {
        for (effect_type, active_status_effect) in status_effects.active.iter() {
            let visible_status_effect = &mut visible_status_effects.effects[effect_type];

            if let Some(active_status_effect) = active_status_effect {
                if let Some((visible_status_effect_id, visible_status_effect_entity)) =
                    visible_status_effect.as_ref()
                {
                    if *visible_status_effect_id == active_status_effect.id {
                        continue;
                    }

                    commands
                        .entity(*visible_status_effect_entity)
                        .despawn_recursive();
                    *visible_status_effect = None;
                }

                if let Some(status_effect_data) = game_data
                    .status_effects
                    .get_status_effect(active_status_effect.id)
                {
                    if let Some(effect_file_id) = status_effect_data.effect_file_id {
                        let effect_entity = commands
                            .spawn((
                                VisibleStatusEffect {
                                    status_effect_type: effect_type,
                                },
                                Transform::default(),
                                GlobalTransform::default(),
                                Visibility::default(),
                                ComputedVisibility::default(),
                            ))
                            .id();

                        spawn_effect_events.send(SpawnEffectEvent::InEntity(
                            effect_entity,
                            SpawnEffectData::with_file_id(effect_file_id).manual_despawn(true),
                        ));

                        commands.entity(entity).add_child(effect_entity);
                        *visible_status_effect = Some((active_status_effect.id, effect_entity));
                    }
                }
            } else if let Some((_, visible_status_effect_entity)) = visible_status_effect.take() {
                commands
                    .entity(visible_status_effect_entity)
                    .despawn_recursive();
            }
        }
    }
}
