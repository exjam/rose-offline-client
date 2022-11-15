use std::time::Duration;

use bevy::{
    ecs::query::WorldQuery,
    prelude::{
        AssetServer, Commands, Entity, EventReader, EventWriter, GlobalTransform, Query, Res,
        Transform,
    },
    time::Time,
};

use rose_data::ItemType;
use rose_game_common::components::{StatusEffects, StatusEffectsRegen};

use crate::{
    audio::SpatialSound,
    components::{PlayerCharacter, SoundCategory},
    events::{SpawnEffectData, SpawnEffectEvent, UseItemEvent},
    resources::{GameData, SoundSettings},
};

#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct EntityQuery<'w> {
    entity: Entity,
    global_transform: &'w GlobalTransform,
    status_effects: &'w mut StatusEffects,
    status_effects_regen: &'w mut StatusEffectsRegen,
    is_player: Option<&'w PlayerCharacter>,
}

pub fn use_item_event_system(
    mut commands: Commands,
    mut events: EventReader<UseItemEvent>,
    mut spawn_effect_events: EventWriter<SpawnEffectEvent>,
    mut query: Query<EntityQuery>,
    asset_server: Res<AssetServer>,
    game_data: Res<GameData>,
    sound_settings: Res<SoundSettings>,
    time: Res<Time>,
) {
    for UseItemEvent { entity, item } in events.iter() {
        let mut user = if let Ok(user) = query.get_mut(*entity) {
            user
        } else {
            continue;
        };

        if item.item_type != ItemType::Consumable {
            continue;
        }

        let item_data =
            if let Some(item_data) = game_data.items.get_consumable_item(item.item_number) {
                item_data
            } else {
                continue;
            };

        if let Some(effect_file_id) = item_data.effect_file_id {
            spawn_effect_events.send(SpawnEffectEvent::OnEntity(
                user.entity,
                None,
                SpawnEffectData::with_file_id(effect_file_id),
            ));
        }

        if let Some(sound_data) = item_data
            .effect_sound_id
            .and_then(|id| game_data.sounds.get_sound(id))
        {
            let category = if user.is_player.is_some() {
                SoundCategory::PlayerCombat
            } else {
                SoundCategory::OtherCombat
            };

            commands.spawn((
                category,
                sound_settings.gain(category),
                SpatialSound::new(asset_server.load(sound_data.path.path())),
                Transform::from_translation(user.global_transform.translation()),
                GlobalTransform::from_translation(user.global_transform.translation()),
            ));
        }

        if let Some((base_status_effect_id, total_potion_value)) = item_data.apply_status_effect {
            if let Some(base_status_effect) = game_data
                .status_effects
                .get_status_effect(base_status_effect_id)
            {
                for (status_effect_data, &potion_value_per_second) in base_status_effect
                    .apply_status_effects
                    .iter()
                    .filter_map(|(id, value)| {
                        game_data
                            .status_effects
                            .get_status_effect(*id)
                            .map(|data| (data, value))
                    })
                {
                    if user
                        .status_effects
                        .can_apply(status_effect_data, status_effect_data.id.get() as i32)
                    {
                        user.status_effects.apply_potion(
                            &mut user.status_effects_regen,
                            status_effect_data,
                            time.last_update().unwrap()
                                + Duration::from_micros(
                                    total_potion_value as u64 * 1000000
                                        / potion_value_per_second as u64,
                                ),
                            total_potion_value,
                            potion_value_per_second,
                        );
                    }
                }
            }
        } else if let Some((_add_ability_type, _add_ability_value)) = item_data.add_ability {
            /*
            TODO:
            ability_values_add_value(
                add_ability_type,
                add_ability_value,
                Some(user.ability_values),
                Some(&mut user.basic_stats),
                Some(&mut user.experience_points),
                Some(&mut user.health_points),
                Some(&mut user.inventory),
                Some(&mut user.mana_points),
                Some(&mut user.skill_points),
                Some(&mut user.stamina),
                Some(&mut user.stat_points),
                Some(&mut user.union_membership),
                user.game_client,
            );
            */
        }
    }
}
