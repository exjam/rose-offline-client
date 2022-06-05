use std::ops::RangeInclusive;

use bevy::{
    math::{Vec3, Vec4},
    prelude::{GlobalTransform, Query, Res, Time, Transform},
};
use rand::Rng;

use rose_file_readers::{PtlKeyframeData, PtlUpdateCoords};

use crate::{
    components::{ActiveParticle, ParticleSequence},
    render::ParticleRenderData,
};

fn rng_gen_range<R: Rng>(rng: &mut R, range: &RangeInclusive<f32>) -> f32 {
    // This function is intentionally written this way to match the
    // original ROSE engine code to behave the same when fmin > fmax
    let fmin = *range.start();
    let fmax = *range.end();

    if fmin == fmax {
        return fmin;
    }

    let frandom = rng.gen_range(0.0..=1.0);
    (frandom * (fmax - fmin).abs()) + fmin
}

fn apply_timestep(
    particle_sequence: &mut ParticleSequence,
    particle_index: usize,
    timestep: f32,
) -> bool {
    let particle = &mut particle_sequence.particles[particle_index];

    particle.age += timestep;
    if particle.age >= particle.life {
        return false;
    }

    particle.keyframe_timer += timestep;

    particle.position += particle.velocity * timestep;

    particle.rotation += particle.rotation_step * timestep;
    while particle.rotation > 360.0 {
        particle.rotation -= 360.0;
    }

    particle.size += particle.size_step * timestep;
    particle.color += particle.color_step * timestep;
    particle.velocity += particle.velocity_step * timestep;
    particle.texture_atlas_index += particle.texture_atlas_index_step * timestep;

    true
}

fn apply_keyframes<R: Rng>(
    rng: &mut R,
    particle_sequence: &mut ParticleSequence,
    particle_index: usize,
) {
    let keyframe_timer = particle_sequence.particles[particle_index].keyframe_timer;
    let next_keyframe_index = particle_sequence.particles[particle_index].next_keyframe_index;

    for keyframe in particle_sequence
        .keyframes
        .iter()
        .skip(next_keyframe_index)
        .filter(|keyframe| keyframe.start_time <= keyframe_timer)
    {
        let next_fade_keyframe = keyframe
            .next_fade_keyframe_index
            .map(|fade_index| &particle_sequence.keyframes[fade_index]);
        let particle = &mut particle_sequence.particles[particle_index];

        match &keyframe.data {
            PtlKeyframeData::SizeXY(x_value_range, y_value_range) => {
                if !keyframe.fade {
                    particle.size.x = rng_gen_range(rng, x_value_range);
                    particle.size.y = rng_gen_range(rng, y_value_range);
                }

                if let Some(next_fade_keyframe) = next_fade_keyframe {
                    if let PtlKeyframeData::SizeXY(next_x_value_range, next_y_value_range) =
                        &next_fade_keyframe.data
                    {
                        let dt = next_fade_keyframe.start_time - keyframe.start_time;
                        let next_x = rng_gen_range(rng, next_x_value_range);
                        let next_y = rng_gen_range(rng, next_y_value_range);
                        particle.size_step.x = (next_x - particle.size.x) / dt;
                        particle.size_step.y = (next_y - particle.size.y) / dt;
                    }
                }
            }
            PtlKeyframeData::Timer(value_range) => {
                // Update the timer and next keyframe index, then return so next frame can handle applying events
                particle.keyframe_timer = rng_gen_range(rng, value_range);
                particle.next_keyframe_index = particle_sequence
                    .keyframes
                    .iter()
                    .filter(|keyframe| keyframe.start_time <= particle.keyframe_timer)
                    .count();
                return;
            }
            PtlKeyframeData::Red(value_range) => {
                if !keyframe.fade {
                    particle.color.x = rng_gen_range(rng, value_range);
                }

                if let Some(next_fade_keyframe) = next_fade_keyframe {
                    if let PtlKeyframeData::Red(next_value_range) = &next_fade_keyframe.data {
                        let dt = next_fade_keyframe.start_time - keyframe.start_time;
                        let next_value = rng_gen_range(rng, next_value_range);
                        particle.color_step.x = (next_value - particle.color.x) / dt;
                    }
                }
            }
            PtlKeyframeData::Green(value_range) => {
                if !keyframe.fade {
                    particle.color.y = rng_gen_range(rng, value_range);
                }

                if let Some(next_fade_keyframe) = next_fade_keyframe {
                    if let PtlKeyframeData::Green(next_value_range) = &next_fade_keyframe.data {
                        let dt = next_fade_keyframe.start_time - keyframe.start_time;
                        let next_value = rng_gen_range(rng, next_value_range);
                        particle.color_step.y = (next_value - particle.color.y) / dt;
                    }
                }
            }
            PtlKeyframeData::Blue(value_range) => {
                if !keyframe.fade {
                    particle.color.z = rng_gen_range(rng, value_range);
                }

                if let Some(next_fade_keyframe) = next_fade_keyframe {
                    if let PtlKeyframeData::Blue(next_value_range) = &next_fade_keyframe.data {
                        let dt = next_fade_keyframe.start_time - keyframe.start_time;
                        let next_value = rng_gen_range(rng, next_value_range);
                        particle.color_step.z = (next_value - particle.color.z) / dt;
                    }
                }
            }
            PtlKeyframeData::Alpha(value_range) => {
                if !keyframe.fade {
                    particle.color.w = rng_gen_range(rng, value_range);
                }

                if let Some(next_fade_keyframe) = next_fade_keyframe {
                    if let PtlKeyframeData::Alpha(next_value_range) = &next_fade_keyframe.data {
                        let dt = next_fade_keyframe.start_time - keyframe.start_time;
                        let next_value = rng_gen_range(rng, next_value_range);
                        particle.color_step.w = (next_value - particle.color.w) / dt;
                    }
                }
            }
            PtlKeyframeData::ColourRGBA(red_range, green_range, blue_range, alpha_range) => {
                if !keyframe.fade {
                    particle.color.x = rng_gen_range(rng, red_range);
                    particle.color.y = rng_gen_range(rng, green_range);
                    particle.color.z = rng_gen_range(rng, blue_range);
                    particle.color.w = rng_gen_range(rng, alpha_range);
                }

                if let Some(next_fade_keyframe) = next_fade_keyframe {
                    if let PtlKeyframeData::ColourRGBA(
                        next_red_range,
                        next_green_range,
                        next_blue_range,
                        next_alpha_range,
                    ) = &next_fade_keyframe.data
                    {
                        let dt = next_fade_keyframe.start_time - keyframe.start_time;
                        let next_red = rng_gen_range(rng, next_red_range);
                        let next_green = rng_gen_range(rng, next_green_range);
                        let next_blue = rng_gen_range(rng, next_blue_range);
                        let next_alpha = rng_gen_range(rng, next_alpha_range);
                        particle.color_step.x = (next_red - particle.color.x) / dt;
                        particle.color_step.y = (next_green - particle.color.y) / dt;
                        particle.color_step.z = (next_blue - particle.color.z) / dt;
                        particle.color_step.w = (next_alpha - particle.color.w) / dt;
                    }
                }
            }
            PtlKeyframeData::VelocityX(value_range) => {
                if !keyframe.fade {
                    particle.velocity.x = rng_gen_range(rng, value_range);
                }

                if let Some(next_fade_keyframe) = next_fade_keyframe {
                    if let PtlKeyframeData::VelocityX(next_value_range) = &next_fade_keyframe.data {
                        let dt = next_fade_keyframe.start_time - keyframe.start_time;
                        let next_value = rng_gen_range(rng, next_value_range);
                        particle.velocity_step.x = (next_value - particle.velocity.x) / dt;
                    }
                }
            }
            PtlKeyframeData::VelocityY(value_range) => {
                if !keyframe.fade {
                    particle.velocity.y = rng_gen_range(rng, value_range);
                }

                if let Some(next_fade_keyframe) = next_fade_keyframe {
                    if let PtlKeyframeData::VelocityY(next_value_range) = &next_fade_keyframe.data {
                        let dt = next_fade_keyframe.start_time - keyframe.start_time;
                        let next_value = rng_gen_range(rng, next_value_range);
                        particle.velocity_step.y = (next_value - particle.velocity.y) / dt;
                    }
                }
            }
            PtlKeyframeData::VelocityZ(value_range) => {
                if !keyframe.fade {
                    particle.velocity.z = rng_gen_range(rng, value_range);
                }

                if let Some(next_fade_keyframe) = next_fade_keyframe {
                    if let PtlKeyframeData::VelocityZ(next_value_range) = &next_fade_keyframe.data {
                        let dt = next_fade_keyframe.start_time - keyframe.start_time;
                        let next_value = rng_gen_range(rng, next_value_range);
                        particle.velocity_step.z = (next_value - particle.velocity.z) / dt;
                    }
                }
            }
            PtlKeyframeData::VelocityXYZ(x_range, y_range, z_range) => {
                if !keyframe.fade {
                    particle.velocity.x = rng_gen_range(rng, x_range);
                    particle.velocity.y = rng_gen_range(rng, y_range);
                    particle.velocity.z = rng_gen_range(rng, z_range);
                }

                if let Some(next_fade_keyframe) = next_fade_keyframe {
                    if let PtlKeyframeData::VelocityXYZ(next_x_range, next_y_range, next_z_range) =
                        &next_fade_keyframe.data
                    {
                        let dt = next_fade_keyframe.start_time - keyframe.start_time;
                        let next_x = rng_gen_range(rng, next_x_range);
                        let next_y = rng_gen_range(rng, next_y_range);
                        let next_z = rng_gen_range(rng, next_z_range);
                        particle.velocity_step.x = (next_x - particle.velocity.x) / dt;
                        particle.velocity_step.y = (next_y - particle.velocity.y) / dt;
                        particle.velocity_step.z = (next_z - particle.velocity.z) / dt;
                    }
                }
            }
            PtlKeyframeData::Texture(value_range) => {
                if !keyframe.fade {
                    particle.texture_atlas_index = rng_gen_range(rng, value_range);
                }

                if let Some(next_fade_keyframe) = next_fade_keyframe {
                    if let PtlKeyframeData::Texture(next_value_range) = &next_fade_keyframe.data {
                        let dt = next_fade_keyframe.start_time - keyframe.start_time;
                        let next_value = rng_gen_range(rng, next_value_range);
                        particle.texture_atlas_index_step =
                            (next_value - particle.texture_atlas_index) / dt;
                    }
                }
            }
            PtlKeyframeData::Rotation(value_range) => {
                if !keyframe.fade {
                    particle.rotation = rng_gen_range(rng, value_range);
                }

                if let Some(next_fade_keyframe) = next_fade_keyframe {
                    if let PtlKeyframeData::Rotation(next_value_range) = &next_fade_keyframe.data {
                        let dt = next_fade_keyframe.start_time - keyframe.start_time;
                        let next_value = rng_gen_range(rng, next_value_range);
                        particle.rotation_step = (next_value - particle.rotation) / dt;
                    }
                }
            }
        }

        particle_sequence.particles[particle_index].next_keyframe_index += 1;
    }
}

pub fn particle_sequence_system(
    time: Res<Time>,
    mut query: Query<(
        &GlobalTransform,
        &mut ParticleSequence,
        &mut ParticleRenderData,
    )>,
) {
    let mut rng = rand::thread_rng();
    let delta_time = time.delta_seconds();

    for (global_transform, mut particle_sequence, mut particle_render_data) in query.iter_mut() {
        if particle_sequence.start_delay > 0.0 {
            particle_sequence.start_delay -= delta_time;
            if particle_sequence.start_delay > 0.0 {
                continue;
            }

            particle_sequence.start_delay = 0.0;
        }

        // Apply particle keyframes
        for particle_index in 0..particle_sequence.particles.len() {
            if apply_timestep(&mut particle_sequence, particle_index, 4.8 * delta_time) {
                let gravity = Vec3::new(
                    rng_gen_range(&mut rng, &particle_sequence.gravity_x),
                    rng_gen_range(&mut rng, &particle_sequence.gravity_y),
                    rng_gen_range(&mut rng, &particle_sequence.gravity_z),
                );
                particle_sequence.particles[particle_index].velocity += gravity * delta_time;

                apply_keyframes(&mut rng, &mut particle_sequence, particle_index);
            }
        }

        // Cleanup any dead particles
        particle_sequence
            .particles
            .retain(|particle| particle.age < particle.life);

        // Spawn any new particles
        if !particle_sequence.finished {
            particle_sequence.emit_counter +=
                delta_time * rng_gen_range(&mut rng, &particle_sequence.emit_rate);

            if particle_sequence.num_loops > 0 {
                let particle_limit = particle_sequence.num_loops * particle_sequence.num_particles;
                let remaining_particles = particle_limit as usize - particle_sequence.num_emitted;
                if particle_sequence.emit_counter as usize >= remaining_particles {
                    particle_sequence.finished = true;
                    particle_sequence.emit_counter = remaining_particles as f32 + 0.1;
                }
            }

            // Spawn new particles
            while particle_sequence.emit_counter > 1.0
                && particle_sequence.particles.len() < particle_sequence.num_particles as usize
            {
                let mut position = Vec3::new(
                    rng_gen_range(&mut rng, &particle_sequence.emit_radius_x),
                    rng_gen_range(&mut rng, &particle_sequence.emit_radius_y),
                    rng_gen_range(&mut rng, &particle_sequence.emit_radius_z),
                );
                if matches!(particle_sequence.update_coords, PtlUpdateCoords::World) {
                    position.x += global_transform.translation.x * 100.0;
                    position.y += global_transform.translation.z * -100.0;
                    position.z += global_transform.translation.y * 100.0;
                }

                let life = rng_gen_range(&mut rng, &particle_sequence.particle_life);
                let particle_index = particle_sequence.particles.len();
                particle_sequence
                    .particles
                    .push(ActiveParticle::new(life, position));

                // Apply initial keyframes
                apply_keyframes(&mut rng, &mut particle_sequence, particle_index);

                particle_sequence.num_emitted += 1;
                particle_sequence.emit_counter -= 1.0;
            }
        }

        // Update render data
        let render_transform = match particle_sequence.update_coords {
            PtlUpdateCoords::World => Transform::default(),
            PtlUpdateCoords::LocalPosition => {
                Transform::from_translation(global_transform.translation)
            }
            PtlUpdateCoords::Local => (*global_transform).into(),
        };
        let texture_atlas_total =
            particle_sequence.texture_atlas_cols * particle_sequence.texture_atlas_rows;
        let texture_atlas_uv_w = 1.0 / particle_sequence.texture_atlas_cols as f32;
        let texture_atlas_uv_h = 1.0 / particle_sequence.texture_atlas_rows as f32;

        particle_render_data.clear();
        for particle in particle_sequence.particles.iter() {
            // TODO: Do we need to support negative texture index ?
            let texture_atlas_index =
                particle.texture_atlas_index.abs() as u32 % texture_atlas_total;
            let texture_atlas_x = texture_atlas_index % particle_sequence.texture_atlas_cols;
            let texture_atlas_y = texture_atlas_index / particle_sequence.texture_atlas_cols;
            let texture_atlas_uv_x = texture_atlas_x as f32 * texture_atlas_uv_w;
            let texture_atlas_uv_y = texture_atlas_y as f32 * texture_atlas_uv_h;

            particle_render_data.add(
                render_transform.mul_vec3(
                    Vec3::new(
                        particle.position.x,
                        particle.position.z,
                        -particle.position.y,
                    ) / 100.0,
                ),
                particle.rotation.to_radians(),
                particle.size / 100.0,
                particle.color,
                Vec4::new(
                    texture_atlas_uv_x,
                    texture_atlas_uv_y,
                    texture_atlas_uv_x + texture_atlas_uv_w,
                    texture_atlas_uv_y + texture_atlas_uv_h,
                ),
            );
        }

        if particle_sequence.finished && particle_sequence.particles.is_empty() {
            // TODO: Despawn self ?
        }
    }
}
