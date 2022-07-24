use bevy::prelude::{Assets, Changed, Commands, Component, Entity, Handle, Query, Res, ResMut};

use super::{audio_source::AudioSource, streaming_sound::StreamingSound, OddioContext, SoundGain};

enum ControlHandle {
    Stereo(oddio::Handle<oddio::Stop<oddio::Gain<oddio::Stream<[f32; 2]>>>>),
    Mono(oddio::Handle<oddio::Stop<oddio::MonoToStereo<oddio::Gain<oddio::Stream<f32>>>>>),
}

#[allow(dead_code)]
impl ControlHandle {
    pub fn gain_control(&mut self) -> oddio::GainControl {
        match self {
            ControlHandle::Stereo(handle) => handle.control::<oddio::Gain<_>, _>(),
            ControlHandle::Mono(handle) => handle.control::<oddio::Gain<_>, _>(),
        }
    }

    pub fn stop_control(&mut self) -> oddio::StopControl {
        match self {
            ControlHandle::Stereo(handle) => handle.control::<oddio::Stop<_>, _>(),
            ControlHandle::Mono(handle) => handle.control::<oddio::Stop<_>, _>(),
        }
    }
}

#[derive(Component)]
pub struct GlobalSound {
    asset_handle: Handle<AudioSource>,
    repeating: bool,
    control_handle: Option<ControlHandle>,
    streaming_sound: Option<StreamingSound>,
}

#[allow(dead_code)]
impl GlobalSound {
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

pub fn global_sound_gain_changed_system(
    mut query: Query<(&mut GlobalSound, &SoundGain), Changed<SoundGain>>,
) {
    for (mut global_sound, gain) in query.iter_mut() {
        if let Some(handle) = global_sound.control_handle.as_mut() {
            match *gain {
                SoundGain::Decibel(db) => handle.gain_control().set_gain(db),
                SoundGain::Ratio(factor) => handle.gain_control().set_amplitude_ratio(factor),
            }
        }
    }
}

pub fn global_sound_system(
    mut commands: Commands,
    mut player: ResMut<OddioContext>,
    audio: Res<Assets<AudioSource>>,
    mut query_global_sounds: Query<(Entity, &mut GlobalSound, Option<&SoundGain>)>,
) {
    let player = &mut player.mixer;

    for (entity, mut global_sound, sound_gain) in query_global_sounds.iter_mut() {
        let repeating = global_sound.repeating;
        let GlobalSound {
            control_handle,
            streaming_sound,
            ..
        } = &mut *global_sound;

        if let Some(handle) = control_handle.as_mut() {
            let has_more_audio = match handle {
                ControlHandle::Stereo(handle) => {
                    if let Some(streaming_sound) = streaming_sound.as_mut() {
                        streaming_sound
                            .fill_stereo(&mut handle.control::<oddio::Stream<_>, _>(), repeating)
                    } else {
                        false
                    }
                }
                ControlHandle::Mono(handle) => {
                    if let Some(streaming_sound) = streaming_sound.as_mut() {
                        streaming_sound
                            .fill_mono(&mut handle.control::<oddio::Stream<_>, _>(), repeating)
                    } else {
                        false
                    }
                }
            };

            if !has_more_audio {
                commands.entity(entity).despawn();
            }
        } else if let Some(audio_source) = audio.get(&global_sound.asset_handle) {
            let mut streaming_sound = StreamingSound::new(audio_source);
            let channels = streaming_sound.channel_count();
            let sample_rate = streaming_sound.sample_rate();

            global_sound.control_handle = Some(if channels == 2 {
                let stream_signal = oddio::Stream::new(sample_rate, sample_rate as usize / 2);
                let gain_signal = match sound_gain {
                    Some(&SoundGain::Decibel(db)) => oddio::Gain::with_gain(stream_signal, db),
                    Some(&SoundGain::Ratio(factor)) => {
                        oddio::Gain::with_amplitude_ratio(stream_signal, factor)
                    }
                    None => oddio::Gain::new(stream_signal),
                };
                let mut handle = player.control().play(gain_signal);

                streaming_sound
                    .fill_stereo(&mut handle.control::<oddio::Stream<_>, _>(), repeating);

                handle.control::<oddio::Stop<_>, _>().resume();
                ControlHandle::Stereo(handle)
            } else {
                let stream_signal = oddio::Stream::new(sample_rate, sample_rate as usize / 2);
                let gain_signal = match sound_gain {
                    Some(&SoundGain::Decibel(db)) => oddio::Gain::with_gain(stream_signal, db),
                    Some(&SoundGain::Ratio(factor)) => {
                        oddio::Gain::with_amplitude_ratio(stream_signal, factor)
                    }
                    None => oddio::Gain::new(stream_signal),
                };
                let mut handle = player.control().play(oddio::MonoToStereo::new(gain_signal));

                streaming_sound.fill_mono(&mut handle.control::<oddio::Stream<_>, _>(), repeating);

                handle.control::<oddio::Stop<_>, _>().resume();
                ControlHandle::Mono(handle)
            });
            global_sound.streaming_sound = Some(streaming_sound);
        }
    }
}
