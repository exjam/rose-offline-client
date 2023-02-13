use bevy::prelude::{
    AddAsset, App, Component, CoreStage, IntoSystemDescriptor, Plugin, Resource, SystemSet, Vec3,
};

mod audio_source;
mod global_sound;
mod ogg;
mod spatial_sound;
mod streaming_sound;
mod wav;

#[derive(Component)]
pub struct SoundRadius(pub f32);

impl SoundRadius {
    pub fn new(radius: f32) -> Self {
        Self(radius)
    }
}

#[derive(Component)]
pub struct SoundVelocity(pub Vec3);

impl SoundVelocity {
    #[allow(dead_code)]
    pub fn new(velocity: Vec3) -> Self {
        Self(velocity)
    }
}

#[allow(dead_code)]
#[derive(Component, PartialEq, Copy, Clone)]
pub enum SoundGain {
    Decibel(f32), // -n .. +n
    Ratio(f32),   // 0..1
}

impl Default for SoundGain {
    fn default() -> Self {
        SoundGain::Ratio(1.0)
    }
}

#[derive(Resource)]
pub struct OddioContext {
    pub mixer: oddio::Handle<oddio::Mixer<[f32; 2]>>,
    pub spatial: oddio::Handle<oddio::SpatialScene>,
    pub sample_rate: u32,
}

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use global_sound::global_sound_system;
use ogg::OggLoader;
use spatial_sound::spatial_sound_system;
use streaming_sound::StreamingSound;
use wav::WavLoader;

pub use audio_source::{AudioSource, StreamingAudioSource};
pub use global_sound::GlobalSound;
pub use spatial_sound::SpatialSound;

use self::{
    global_sound::global_sound_gain_changed_system,
    spatial_sound::spatial_sound_gain_changed_system,
};

pub struct OddioPlugin;

impl Plugin for OddioPlugin {
    fn build(&self, app: &mut App) {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("no output device available");
        let sample_rate = device.default_output_config().unwrap().sample_rate();
        let config = cpal::StreamConfig {
            channels: 2,
            sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        let (mut root_mixer_handle, root_mixer) = oddio::split(oddio::Mixer::new());
        let (scene_handle, scene) = oddio::split(oddio::SpatialScene::new());
        root_mixer_handle.control().play(scene);

        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let frames = oddio::frame_stereo(data);
                    oddio::run(&root_mixer, sample_rate.0, frames);
                },
                move |err| {
                    eprintln!("{}", err);
                },
                None,
            )
            .unwrap();
        stream.play().unwrap();

        app.insert_non_send_resource(stream)
            .insert_resource(OddioContext {
                mixer: root_mixer_handle,
                spatial: scene_handle,
                sample_rate: sample_rate.0,
            })
            .add_asset::<AudioSource>()
            .init_asset_loader::<OggLoader>()
            .init_asset_loader::<WavLoader>()
            .add_system_set_to_stage(
                CoreStage::Last,
                SystemSet::new()
                    .with_system(spatial_sound_gain_changed_system.before(spatial_sound_system))
                    .with_system(spatial_sound_system)
                    .with_system(global_sound_gain_changed_system.before(global_sound_system))
                    .with_system(global_sound_system),
            );
    }
}
