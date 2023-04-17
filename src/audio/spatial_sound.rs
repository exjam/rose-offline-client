use bevy::{
    asset::LoadState,
    math::Vec3,
    prelude::{
        AssetServer, Assets, Camera3d, Changed, Commands, Component, Entity, GlobalTransform,
        Handle, Local, Query, Res, ResMut, With,
    },
    time::Time,
};

use crate::{
    audio::{AudioSource, OddioContext, SoundGain, SoundRadius, StreamingSound},
    components::PlayerCharacter,
};

struct SpatialControlHandle(
    oddio::Handle<oddio::SpatialBuffered<oddio::Stop<oddio::Gain<oddio::Stream<f32>>>>>,
);

#[allow(dead_code)]
impl SpatialControlHandle {
    pub fn gain_control(&mut self) -> oddio::GainControl {
        self.0.control::<oddio::Gain<_>, _>()
    }

    pub fn stop_control(&mut self) -> oddio::StopControl {
        self.0.control::<oddio::Stop<_>, _>()
    }

    pub fn stream_control(&mut self) -> oddio::StreamControl<f32> {
        self.0.control::<oddio::Stream<_>, _>()
    }

    pub fn spatial_control(&mut self) -> oddio::SpatialControl {
        self.0.control::<oddio::SpatialBuffered<_>, _>()
    }
}

#[derive(Component)]
pub struct SpatialSound {
    asset_handle: Handle<AudioSource>,
    repeating: bool,
    control_handle: Option<SpatialControlHandle>,
    streaming_sound: Option<StreamingSound>,
    last_position: Option<Vec3>,
}

#[allow(dead_code)]
impl SpatialSound {
    pub fn new(audio_source: Handle<AudioSource>) -> Self {
        Self {
            asset_handle: audio_source,
            repeating: false,
            control_handle: None,
            streaming_sound: None,
            last_position: None,
        }
    }

    pub fn new_repeating(audio_source: Handle<AudioSource>) -> Self {
        Self {
            asset_handle: audio_source,
            repeating: true,
            control_handle: None,
            streaming_sound: None,
            last_position: None,
        }
    }
}

pub fn spatial_sound_gain_changed_system(
    mut query: Query<(&mut SpatialSound, &SoundGain), Changed<SoundGain>>,
) {
    for (mut spatial_sound, gain) in query.iter_mut() {
        if let Some(handle) = spatial_sound.control_handle.as_mut() {
            match *gain {
                SoundGain::Decibel(db) => handle.gain_control().set_gain(db),
                SoundGain::Ratio(factor) => handle.gain_control().set_amplitude_ratio(factor),
            }
        }
    }
}

pub fn spatial_sound_system(
    mut commands: Commands,
    mut context: ResMut<OddioContext>,
    audio: Res<Assets<AudioSource>>,
    asset_server: Res<AssetServer>,
    camera: Query<&GlobalTransform, With<Camera3d>>,
    mut query_spatial_sounds: Query<(
        Entity,
        &mut SpatialSound,
        &GlobalTransform,
        Option<&SoundRadius>,
        Option<&SoundGain>,
    )>,
    mut last_listener_position: Local<Option<Vec3>>,
    query_player: Query<&GlobalTransform, With<PlayerCharacter>>,
    time: Res<Time>,
) {
    let player = &mut context.spatial;
    let (_, camera_rotation, camera_position) = camera.single().to_scale_rotation_translation();

    // Use player position as listener position (fallback to camera if no player exists)
    let listener_position = query_player
        .get_single()
        .map_or(camera_position, |player_transform| {
            player_transform.translation()
        });

    // Guess listener velocity by distance between current and last position.
    let listener_velocity = if let Some(last_listener_position) = *last_listener_position {
        listener_position - last_listener_position
    } else {
        Vec3::ZERO
    };
    *last_listener_position = Some(listener_position);

    player
        .control()
        .set_listener_rotation(camera_rotation.to_array().into());

    for (entity, mut spatial_sound, global_transform, sound_radius, sound_gain) in
        query_spatial_sounds.iter_mut()
    {
        let repeating = spatial_sound.repeating;
        let SpatialSound {
            control_handle,
            streaming_sound,
            last_position,
            ..
        } = &mut *spatial_sound;

        let sound_global_translation = global_transform.translation();

        let spatial_velocity = {
            // Guess sound velocity by distance between current and last position.
            let sound_velocity = if let Some(last_position) = last_position {
                sound_global_translation - *last_position
            } else {
                Vec3::ZERO
            };
            *last_position = Some(sound_global_translation);

            // Velocity is relative to listener velocity
            let relative_velocity = sound_velocity - listener_velocity;

            // Velocity is in metres per second
            relative_velocity / time.delta_seconds()
        };

        // Adjust spatial position to be in direction of camera, but distance from player
        let spatial_position = (sound_global_translation - camera_position).normalize()
            * (sound_global_translation - listener_position).length();

        if let Some(handle) = control_handle.as_mut() {
            let has_more_audio = if let Some(streaming_sound) = streaming_sound.as_mut() {
                streaming_sound.fill_mono(&mut handle.stream_control(), repeating)
            } else {
                false
            };

            handle.spatial_control().set_motion(
                spatial_position.to_array().into(),
                spatial_velocity.to_array().into(),
                false,
            );

            if !has_more_audio {
                spatial_sound.control_handle = None;
                spatial_sound.asset_handle = Handle::default();
                commands.entity(entity).despawn();
            }
        } else if let Some(audio_source) = audio.get(&spatial_sound.asset_handle) {
            let mut streaming_sound = StreamingSound::new(audio_source);
            let sample_rate = streaming_sound.sample_rate();

            let stream_signal = oddio::Stream::new(sample_rate, sample_rate as usize / 8);
            let gain_signal = match sound_gain {
                Some(&SoundGain::Decibel(db)) => oddio::Gain::with_gain(stream_signal, db),
                Some(&SoundGain::Ratio(factor)) => {
                    oddio::Gain::with_amplitude_ratio(stream_signal, factor)
                }
                None => oddio::Gain::new(stream_signal),
            };

            let mut handle = SpatialControlHandle(player.control().play_buffered(
                gain_signal,
                oddio::SpatialOptions {
                    position: spatial_position.to_array().into(),
                    velocity: spatial_velocity.to_array().into(),
                    radius: sound_radius.map(|x| x.0).unwrap_or(4.0),
                },
                500.0,
                sample_rate,
                0.1,
            ));

            handle.spatial_control().set_motion(
                spatial_position.to_array().into(),
                spatial_velocity.to_array().into(),
                true,
            );

            handle.stop_control().resume();
            streaming_sound.fill_mono(&mut handle.stream_control(), repeating);

            spatial_sound.control_handle = Some(handle);
            spatial_sound.streaming_sound = Some(streaming_sound);
        } else if matches!(
            asset_server.get_load_state(&spatial_sound.asset_handle),
            LoadState::Failed | LoadState::Unloaded
        ) {
            spatial_sound.asset_handle = Handle::default();

            if !spatial_sound.repeating {
                // Despawn non-repeating sounds which fail to load
                commands.entity(entity).despawn();
            }
        }
    }
}
