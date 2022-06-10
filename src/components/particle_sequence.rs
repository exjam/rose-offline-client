use std::ops::RangeInclusive;

use bevy::{
    math::{Quat, Vec2, Vec3, Vec4},
    prelude::Component,
};
use rand::Rng;

use rose_file_readers::{PtlKeyframeData, PtlSequence, PtlUpdateCoords};

pub struct ActiveParticle {
    pub age: f32,
    pub keyframe_timer: f32,
    pub life: f32,
    pub next_keyframe_index: usize,
    pub gravity_local: Vec3,
    pub world_direction: Option<Quat>,

    pub position: Vec3,

    pub rotation: f32, // degrees
    pub rotation_step: f32,

    pub size: Vec2,
    pub size_step: Vec2,

    pub color: Vec4,
    pub color_step: Vec4,

    pub velocity: Vec3,
    pub velocity_step: Vec3,

    pub texture_atlas_index: f32,
    pub texture_atlas_index_step: f32,
}

impl ActiveParticle {
    pub fn new(
        life: f32,
        position: Vec3,
        gravity_local: Vec3,
        world_direction: Option<Quat>,
    ) -> Self {
        Self {
            age: 0.0,
            keyframe_timer: 0.0,
            life,
            next_keyframe_index: 0,
            gravity_local,
            world_direction,
            position,

            rotation: 0.0,
            size: Vec2::new(10.0, 10.0),
            color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            velocity: Vec3::new(0.0, 0.0, 0.0),
            texture_atlas_index: 0.0,

            rotation_step: Default::default(),
            size_step: Default::default(),
            color_step: Default::default(),
            velocity_step: Default::default(),
            texture_atlas_index_step: Default::default(),
        }
    }
}

#[derive(Debug)]
pub struct ParticleSequenceKeyframe {
    pub start_time: f32,
    pub fade: bool,
    pub next_fade_keyframe_index: Option<usize>,
    pub data: PtlKeyframeData,
}

#[derive(Component)]
pub struct ParticleSequence {
    pub emit_rate: RangeInclusive<f32>,
    pub particle_life: RangeInclusive<f32>,
    pub emit_radius_x: RangeInclusive<f32>,
    pub emit_radius_y: RangeInclusive<f32>,
    pub emit_radius_z: RangeInclusive<f32>,
    pub gravity_x: RangeInclusive<f32>,
    pub gravity_y: RangeInclusive<f32>,
    pub gravity_z: RangeInclusive<f32>,
    pub keyframes: Vec<ParticleSequenceKeyframe>,
    pub texture_atlas_cols: u32,
    pub texture_atlas_rows: u32,
    pub update_coords: PtlUpdateCoords,
    pub num_loops: u32,
    pub num_particles: u32,

    pub start_delay: f32,
    pub emit_counter: f32,
    pub num_emitted: usize,
    pub particles: Vec<ActiveParticle>,

    pub finished: bool,
}

impl ParticleSequence {
    pub fn from(sequence: PtlSequence) -> Self {
        let mut rng = rand::thread_rng();

        // Select key frame start times
        let mut keyframes: Vec<ParticleSequenceKeyframe> = sequence
            .keyframes
            .into_iter()
            .map(|keyframe| ParticleSequenceKeyframe {
                start_time: rng.gen_range(keyframe.start_time),
                fade: keyframe.fade,
                next_fade_keyframe_index: None,
                data: keyframe.data,
            })
            .collect();
        keyframes.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap());

        for i in 0..keyframes.len() {
            let current = &keyframes[i];

            // Find the first next event of the same type with "fade"
            for j in i + 1..keyframes.len() {
                let next = &keyframes[j];
                if next.fade
                    && std::mem::discriminant(&current.data) == std::mem::discriminant(&next.data)
                {
                    keyframes[i].next_fade_keyframe_index = Some(j);
                    break;
                }
            }
        }

        Self {
            keyframes,
            emit_counter: 0.0,
            num_emitted: 0,
            particles: Vec::with_capacity(sequence.num_particles as usize),
            finished: false,
            emit_rate: sequence.emit_rate,
            particle_life: sequence.life,
            emit_radius_x: sequence.emit_radius_x,
            emit_radius_y: sequence.emit_radius_y,
            emit_radius_z: sequence.emit_radius_z,
            gravity_x: sequence.gravity_x,
            gravity_y: sequence.gravity_y,
            gravity_z: sequence.gravity_z,
            texture_atlas_cols: sequence.texture_atlas_cols,
            texture_atlas_rows: sequence.texture_atlas_rows,
            update_coords: sequence.update_coords,
            num_loops: sequence.num_loops,
            num_particles: sequence.num_particles,
            start_delay: 0.0,
        }
    }

    pub fn with_start_delay(mut self, start_delay: f32) -> Self {
        self.start_delay = start_delay;
        self
    }
}
