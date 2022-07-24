use bevy::{
    math::Vec3,
    prelude::{AssetServer, Assets, Commands, EventReader, GlobalTransform, Query, Res, Transform},
};

use rose_data::{AnimationEventFlags, SoundId};

use crate::{
    audio::SpatialSound,
    components::{PlayerCharacter, SoundCategory},
    events::AnimationFrameEvent,
    resources::{CurrentZone, GameData, SoundSettings},
    zone_loader::ZoneLoaderAsset,
};

pub fn animation_sound_system(
    mut commands: Commands,
    mut animation_frame_events: EventReader<AnimationFrameEvent>,
    game_data: Res<GameData>,
    asset_server: Res<AssetServer>,
    current_zone: Option<Res<CurrentZone>>,
    zone_loader_assets: Res<Assets<ZoneLoaderAsset>>,
    sound_settings: Res<SoundSettings>,
    query_global_transform: Query<&GlobalTransform>,
    query_player: Query<&PlayerCharacter>,
) {
    for event in animation_frame_events.iter() {
        if event.flags.contains(AnimationEventFlags::SOUND_FOOTSTEP) {
            let (sound_category, sound_gain) = if query_player.get(event.entity).is_ok() {
                (
                    SoundCategory::PlayerFootstep,
                    sound_settings.gain(SoundCategory::PlayerFootstep),
                )
            } else {
                (
                    SoundCategory::OtherFootstep,
                    sound_settings.gain(SoundCategory::OtherFootstep),
                )
            };

            if let Ok(global_transform) = query_global_transform.get(event.entity) {
                let translation = global_transform.translation;
                let position =
                    Vec3::new(translation.x * 100.0, -translation.z * 100.0, translation.y);
                let default_step_sound_data =
                    game_data.sounds.get_sound(SoundId::new(653).unwrap());

                let step_sound_data = if let Some(current_zone) = current_zone.as_ref() {
                    if let Some(current_zone_data) = zone_loader_assets.get(&current_zone.handle) {
                        if current_zone_data.get_terrain_height(position.x, position.y) / 100.0
                            < (translation.y - 0.1)
                        {
                            // Standing on an object, use default sound
                            default_step_sound_data
                        } else {
                            let tile_number =
                                current_zone_data.get_tile_index(position.x, position.y);
                            let zone_type = game_data
                                .zone_list
                                .get_zone(current_zone.id)
                                .and_then(|zone_data| zone_data.footstep_type)
                                .unwrap_or(0) as usize;
                            game_data.sounds.get_step_sound(tile_number, zone_type)
                        }
                    } else {
                        default_step_sound_data
                    }
                } else {
                    default_step_sound_data
                };

                if let Some(sound_data) = step_sound_data {
                    commands.spawn_bundle((
                        sound_category,
                        sound_gain,
                        SpatialSound::new(asset_server.load(sound_data.path.path())),
                        Transform::from_translation(translation),
                        GlobalTransform::from_translation(translation),
                    ));
                }
            }
        }
    }
}
