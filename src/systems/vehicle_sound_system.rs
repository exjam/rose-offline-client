use bevy::prelude::{
    AssetServer, BuildChildren, Changed, Commands, GlobalTransform, Or, Query, Res, Transform,
};
use enum_map::enum_map;
use rose_data::{VehiclePartIndex, VehicleType};

use crate::{
    audio::SpatialSound,
    components::{
        Command, PlayerCharacter, SoundCategory, Vehicle, VehicleModel, VehicleSound,
        VehicleSoundState,
    },
    resources::GameData,
};

pub fn vehicle_sound_system(
    mut commands: Commands,
    mut query: Query<
        (&Command, &Vehicle, Option<&PlayerCharacter>),
        Or<(Changed<Command>, Changed<Vehicle>)>,
    >,
    mut query_vehicle_model: Query<(
        &GlobalTransform,
        &mut VehicleModel,
        Option<&mut VehicleSound>,
    )>,
    asset_server: Res<AssetServer>,
    game_data: Res<GameData>,
) {
    for (command, vehicle, player) in query.iter_mut() {
        let (global_transform, mut vehicle_model, vehicle_sound) = query_vehicle_model
            .get_mut(vehicle.vehicle_model_entity)
            .unwrap();

        let vehicle_sound_state = match command {
            Command::Move(_) => VehicleSoundState::Move,
            _ => VehicleSoundState::Idle,
        };

        let sound_category = if player.is_some() {
            SoundCategory::PlayerFootstep
        } else {
            SoundCategory::OtherFootstep
        };

        if let Some(mut vehicle_sound) = vehicle_sound {
            if vehicle_sound.state == vehicle_sound_state {
                // Correct sound is playing, do nothing
                continue;
            }
            vehicle_sound.state = vehicle_sound_state;

            for (vehicle_part_index, sound_entity) in vehicle_sound.sound_entity.iter() {
                // Remove previous sound
                commands.entity(*sound_entity).remove::<SpatialSound>();

                let sound_data = if let Some(item_data) = game_data
                    .items
                    .get_vehicle_item(vehicle_model.model_parts[vehicle_part_index].0)
                {
                    if matches!(vehicle_part_index, VehiclePartIndex::Leg)
                        && matches!(item_data.vehicle_type, VehicleType::CastleGear)
                    {
                        // Skip castle gear leg noise, as that sound should originate from the animation frame events
                        None
                    } else {
                        let sound_id = match vehicle_sound_state {
                            VehicleSoundState::Move => item_data.move_sound_id,
                            VehicleSoundState::Idle => item_data.stop_sound_id,
                        };

                        sound_id.and_then(|id| game_data.sounds.get_sound(id))
                    }
                } else {
                    None
                };

                // Add new sound
                if let Some(sound_data) = sound_data {
                    commands
                        .entity(*sound_entity)
                        .insert(SpatialSound::new_repeating(
                            asset_server.load(sound_data.path.path()),
                        ));
                }
            }
        } else {
            let sound_entity = enum_map! {
                vehicle_part_index => {
                    let sound_data = if let Some(item_data) = game_data.items.get_vehicle_item(vehicle_model.model_parts[vehicle_part_index].0) {
                        if matches!(vehicle_part_index, VehiclePartIndex::Leg) && matches!(item_data.vehicle_type, VehicleType::CastleGear) {
                            // Skip castle gear leg noise, as that sound should originate from the animation frame events
                            None
                        } else {
                            let sound_id = match vehicle_sound_state {
                                VehicleSoundState::Move => item_data.move_sound_id,
                                VehicleSoundState::Idle => item_data.stop_sound_id,
                            };

                            sound_id.and_then(|id| game_data.sounds.get_sound(id))
                        }
                    } else {
                        None
                    };

                    let sound_entity = if let Some(sound_data) = sound_data {
                        commands.spawn((
                            SpatialSound::new_repeating(asset_server.load(sound_data.path.path())),
                            sound_category,
                            Transform::default(),
                            *global_transform,
                        )).id()
                    } else {
                        commands.spawn((
                            sound_category,
                            Transform::default(),
                            *global_transform,
                        )).id()
                    };

                    commands.entity(vehicle.vehicle_model_entity).add_child(sound_entity);
                    vehicle_model.model_parts[vehicle_part_index].1.push(sound_entity);
                    sound_entity
                }
            };

            commands
                .entity(vehicle.vehicle_model_entity)
                .insert(VehicleSound {
                    state: vehicle_sound_state,
                    sound_entity,
                });
        }
    }
}
