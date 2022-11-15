use bevy::{
    hierarchy::BuildChildren,
    math::{Quat, Vec3},
    prelude::{
        AssetServer, Assets, Commands, ComputedVisibility, Entity, GlobalTransform, Mesh,
        Transform, Visibility,
    },
    render::{
        primitives::Aabb,
        render_resource::{BlendFactor, BlendOperation},
        view::NoFrustumCulling,
    },
};
use rose_file_readers::{EftFile, EftMesh, EftParticle, PtlFile, VfsPath, VirtualFilesystem};

use crate::{
    components::{Effect, EffectMesh, EffectParticle, ParticleSequence},
    render::{
        EffectMeshMaterial, ParticleMaterial, ParticleRenderBillboardType, ParticleRenderData,
        RgbTextureLoader,
    },
};

pub fn spawn_effect(
    vfs: &VirtualFilesystem,
    commands: &mut Commands,
    asset_server: &AssetServer,
    particle_materials: &mut Assets<ParticleMaterial>,
    effect_mesh_materials: &mut Assets<EffectMeshMaterial>,
    effect_path: VfsPath,
    manual_despawn: bool,
    effect_entity: Option<Entity>,
) -> Option<Entity> {
    let eft_file = vfs.read_file::<EftFile, _>(effect_path).ok()?;

    let mut child_entities = Vec::with_capacity(eft_file.particles.len());
    for eft_particle in eft_file.particles {
        if let Some(particle_entity) = spawn_particle(
            vfs,
            commands,
            asset_server,
            particle_materials,
            &eft_particle,
        ) {
            child_entities.push(particle_entity);
        }
    }

    for eft_particle in eft_file.meshes {
        if let Some(mesh_entity) =
            spawn_mesh(commands, asset_server, effect_mesh_materials, &eft_particle)
        {
            child_entities.push(mesh_entity);
        }
    }

    // TODO: eft_file.sound_file
    // TODO: eft_file.sound_repeat_count
    if let Some(effect_entity) = effect_entity {
        commands
            .entity(effect_entity)
            .insert(Effect::new(manual_despawn))
            .push_children(&child_entities);
        Some(effect_entity)
    } else {
        let root_entity = commands
            .spawn((
                Effect::new(manual_despawn),
                Transform::default(),
                GlobalTransform::default(),
                Visibility::default(),
                ComputedVisibility::default(),
            ))
            .push_children(&child_entities)
            .id();

        Some(root_entity)
    }
}

fn decode_blend_op(value: u32) -> BlendOperation {
    match value {
        1 => BlendOperation::Add,
        2 => BlendOperation::Subtract,
        3 => BlendOperation::ReverseSubtract,
        4 => BlendOperation::Min,
        5 => BlendOperation::Max,
        _ => BlendOperation::Add,
    }
}

fn decode_blend_factor(value: u32) -> BlendFactor {
    match value {
        1 => BlendFactor::Zero,
        2 => BlendFactor::One,
        3 => BlendFactor::Src,
        4 => BlendFactor::OneMinusSrc,
        5 => BlendFactor::SrcAlpha,
        6 => BlendFactor::OneMinusSrcAlpha,
        7 => BlendFactor::DstAlpha,
        8 => BlendFactor::OneMinusDstAlpha,
        9 => BlendFactor::Dst,
        10 => BlendFactor::OneMinusDst,
        11 => BlendFactor::SrcAlphaSaturated,
        _ => BlendFactor::Zero,
    }
}

fn spawn_mesh(
    commands: &mut Commands,
    asset_server: &AssetServer,
    effect_mesh_materials: &mut Assets<EffectMeshMaterial>,
    eft_mesh: &EftMesh,
) -> Option<Entity> {
    Some(
        commands
            .spawn((
                Transform::from_translation(
                    Vec3::new(
                        eft_mesh.position.x,
                        eft_mesh.position.z,
                        -eft_mesh.position.y,
                    ) / 100.0,
                )
                .with_rotation(
                    Quat::from_axis_angle(Vec3::Y, eft_mesh.yaw.to_radians())
                        * Quat::from_axis_angle(Vec3::X, eft_mesh.pitch.to_radians())
                        * Quat::from_axis_angle(Vec3::Z, eft_mesh.roll.to_radians()),
                ),
                GlobalTransform::default(),
                Visibility::default(),
                ComputedVisibility::default(),
            ))
            .with_children(|child_builder| {
                child_builder.spawn((
                    EffectMesh {},
                    asset_server.load::<Mesh, _>(eft_mesh.mesh_file.path()),
                    effect_mesh_materials.add(EffectMeshMaterial {
                        base_texture: Some(asset_server.load(RgbTextureLoader::convert_path(
                            eft_mesh.mesh_texture_file.path(),
                        ))),
                        alpha_enabled: eft_mesh.alpha_enabled,
                        alpha_test: eft_mesh.alpha_test_enabled,
                        two_sided: eft_mesh.two_sided,
                        z_test_enabled: eft_mesh.depth_test_enabled,
                        z_write_enabled: eft_mesh.depth_write_enabled,
                        src_blend_factor: decode_blend_factor(eft_mesh.src_blend_factor),
                        dst_blend_factor: decode_blend_factor(eft_mesh.dst_blend_factor),
                        blend_op: decode_blend_op(eft_mesh.blend_op),
                    }),
                    Visibility::default(),
                    ComputedVisibility::default(),
                    Transform::default(),
                    GlobalTransform::default(),
                ));

                /*
                TODO: Support morph target animations

                if let Some(mesh_animation_file) = eft_mesh.mesh_animation_file.as_ref() {
                    entity_comands.insert(ActiveMotion {
                        motion: asset_server.load(mesh_animation_file.path()),
                        repeat_limit: if eft_mesh.repeat_count == 0 {
                            None
                        } else {
                            Some(eft_mesh.repeat_count as usize)
                        },
                        complete: false,
                        animation_speed: 1.0,
                        start_time: None,
                    });
                }
                */

                // TODO: eft_mesh.start_delay
                // TODO: eft_mesh.is_linked
                // TODO: eft_mesh.animation_file
                // TODO: eft_mesh.animation_repeat_count
            })
            .id(),
    )
}

fn spawn_particle(
    vfs: &VirtualFilesystem,
    commands: &mut Commands,
    asset_server: &AssetServer,
    particle_materials: &mut Assets<ParticleMaterial>,
    eft_particle: &EftParticle,
) -> Option<Entity> {
    let ptl_file = vfs
        .read_file::<PtlFile, _>(&eft_particle.particle_file)
        .ok()?;

    // TODO: eft_particle.is_linked
    // TODO: eft_particle.animation_file / eft_particle.animation_repeat_count

    Some(
        commands
            .spawn((
                Transform::from_translation(
                    Vec3::new(
                        eft_particle.position.x,
                        eft_particle.position.z,
                        -eft_particle.position.y,
                    ) / 100.0,
                )
                .with_rotation(
                    Quat::from_axis_angle(Vec3::Y, eft_particle.yaw.to_radians())
                        * Quat::from_axis_angle(Vec3::X, eft_particle.pitch.to_radians())
                        * Quat::from_axis_angle(Vec3::Z, eft_particle.roll.to_radians()),
                ),
                GlobalTransform::default(),
                Visibility::default(),
                ComputedVisibility::default(),
            ))
            .with_children(|child_builder| {
                for sequence in ptl_file.sequences {
                    child_builder.spawn((
                        EffectParticle {},
                        ParticleRenderData::new(
                            sequence.num_particles as usize,
                            sequence.blend_op as u8,
                            sequence.src_blend_mode as u8,
                            sequence.dst_blend_mode as u8,
                            match sequence.align_type {
                                0 => ParticleRenderBillboardType::Full,
                                1 => ParticleRenderBillboardType::None,
                                2 => ParticleRenderBillboardType::YAxis,
                                _ => ParticleRenderBillboardType::Full,
                            },
                        ),
                        particle_materials.add(ParticleMaterial {
                            texture: asset_server
                                .load(RgbTextureLoader::convert_path(sequence.texture_path.path())),
                        }),
                        ParticleSequence::from(sequence)
                            .with_start_delay(eft_particle.start_delay as f32 / 1000.0),
                        Transform::default(),
                        GlobalTransform::default(),
                        Aabb::default(),
                        Visibility::default(),
                        ComputedVisibility::default(),
                        NoFrustumCulling, // AABB culling is broken for particles
                    ));
                }
            })
            .id(),
    )
}
