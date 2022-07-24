use bevy::{
    math::Vec3,
    prelude::{
        Assets, Camera3d, Changed, Commands, Component, Entity, GlobalTransform, Handle, Local,
        Query, Res, ResMut, With,
    },
};

use super::{AudioSource, OddioContext, SoundGain, SoundRadius, SoundVelocity, StreamingSound};

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
}

#[allow(dead_code)]
impl SpatialSound {
    pub fn new(audio_source: Handle<AudioSource>) -> Self {
        Self {
            asset_handle: audio_source,
            repeating: false,
            control_handle: None,
            streaming_sound: None,
        }
    }

    pub fn new_repeating(audio_source: Handle<AudioSource>) -> Self {
        Self {
            asset_handle: audio_source,
            repeating: true,
            control_handle: None,
            streaming_sound: None,
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

#[derive(Default)]
pub struct CameraLastPosition {
    pub position: Option<Vec3>,
}

pub fn spatial_sound_system(
    mut commands: Commands,
    mut context: ResMut<OddioContext>,
    audio: Res<Assets<AudioSource>>,
    camera: Query<&GlobalTransform, With<Camera3d>>,
    mut query_spatial_sounds: Query<(
        Entity,
        &mut SpatialSound,
        &GlobalTransform,
        Option<&SoundRadius>,
        Option<&SoundVelocity>,
        Option<&SoundGain>,
    )>,
    mut last_camera_position: Local<CameraLastPosition>,
) {
    let player = &mut context.spatial;
    let listener_transform = camera.single();
    let listener_rotation = listener_transform.rotation;
    let listener_position = listener_transform.translation;

    // We just guess velocity based on cameras last movement...
    // TODO: This will be garbage on teleport
    let listener_velocity = if let Some(last_listener_position) = last_camera_position.position {
        listener_position - last_listener_position
    } else {
        Vec3::ZERO
    };
    last_camera_position.position = Some(listener_position);

    player
        .control()
        .set_listener_rotation(listener_rotation.to_array().into());

    for (entity, mut spatial_sound, global_transform, sound_radius, sound_velocity, sound_gain) in
        query_spatial_sounds.iter_mut()
    {
        let velocity = sound_velocity.map(|v| v.0).unwrap_or(Vec3::ZERO) - listener_velocity;
        let position = global_transform.translation - listener_position;
        let repeating = spatial_sound.repeating;
        let SpatialSound {
            control_handle,
            streaming_sound,
            ..
        } = &mut *spatial_sound;

        if let Some(handle) = control_handle.as_mut() {
            let has_more_audio = if let Some(streaming_sound) = streaming_sound.as_mut() {
                streaming_sound.fill_mono(&mut handle.stream_control(), repeating)
            } else {
                false
            };

            handle.spatial_control().set_motion(
                position.to_array().into(),
                velocity.to_array().into(),
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
                    position: position.to_array().into(),
                    velocity: velocity.to_array().into(),
                    radius: sound_radius.map(|x| x.0).unwrap_or(4.0),
                },
                500.0,
                sample_rate,
                0.1,
            ));

            handle.spatial_control().set_motion(
                position.to_array().into(),
                velocity.to_array().into(),
                true,
            );

            handle.stop_control().resume();
            streaming_sound.fill_mono(&mut handle.stream_control(), repeating);

            spatial_sound.control_handle = Some(handle);
            spatial_sound.streaming_sound = Some(streaming_sound);
        }
    }
}
