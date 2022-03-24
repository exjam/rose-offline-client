use bevy::{
    core::Time,
    math::{Quat, Vec3, Vec3A},
    prelude::{
        AssetServer, Assets, Changed, Commands, Component, Entity, GlobalTransform, Handle, Query,
        Res, ResMut, Transform, With, Without,
    },
    render::{
        mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
        primitives::Aabb,
    },
};

use bevy_rapier3d::{
    physics::ColliderBundle,
    prelude::{
        ColliderFlags, ColliderFlagsComponent, ColliderPosition, ColliderPositionComponent,
        ColliderShape, ColliderShapeComponent, InteractionGroups,
    },
};
use rose_game_common::components::{MoveMode, Npc};

use crate::{
    components::{
        ActiveMotion, Command, CommandData, NpcModel, COLLISION_FILTER_CLICKABLE,
        COLLISION_FILTER_INSPECTABLE, COLLISION_GROUP_NPC,
    },
    model_loader::ModelLoader,
    render::StaticMeshMaterial,
    resources::GameData,
    zmo_asset_loader::ZmoAsset,
};

#[derive(Component)]
pub struct CommandNpcMotion {
    pub command: CommandData,
    pub move_mode: MoveMode,
}

fn get_command_motion(
    npc_model: &NpcModel,
    move_mode: &MoveMode,
    command: &Command,
) -> Option<Handle<ZmoAsset>> {
    let action_index = match command.command {
        CommandData::Stop => 0,
        CommandData::Move(_) => match move_mode {
            MoveMode::Walk => 1,
            MoveMode::Run => 5,
            _ => 1,
        },
    };

    npc_model
        .action_motions
        .iter()
        .find(|(action_id, _)| *action_id == action_index)
        .or_else(|| npc_model.action_motions.get(0))
        .map(|(_, motion)| motion.clone())
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn npc_model_animation_system(
    mut commands: Commands,
    mut query_command: Query<
        (
            Entity,
            &NpcModel,
            &Command,
            &MoveMode,
            Option<&CommandNpcMotion>,
        ),
        With<SkinnedMesh>,
    >,
    time: Res<Time>,
) {
    for (entity, npc_model, command, move_mode, command_npc_motion) in query_command.iter_mut() {
        if command_npc_motion.map_or(false, |x| {
            std::mem::discriminant(&x.command) == std::mem::discriminant(&command.command)
                && x.move_mode == *move_mode
        }) {
            continue;
        }

        if let Some(motion) = get_command_motion(npc_model, move_mode, command) {
            commands.entity(entity).insert_bundle((
                CommandNpcMotion {
                    command: command.command.clone(),
                    move_mode: *move_mode,
                },
                ActiveMotion::new(motion.clone(), time.seconds_since_startup()),
            ));
        }
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn npc_model_system(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &Npc,
            Option<&Command>,
            Option<&MoveMode>,
            Option<&mut NpcModel>,
            Option<&SkinnedMesh>,
            &Transform,
        ),
        Changed<Npc>,
    >,
    asset_server: Res<AssetServer>,
    model_loader: Res<ModelLoader>,
    mut static_mesh_materials: ResMut<Assets<StaticMeshMaterial>>,
    mut skinned_mesh_inverse_bindposes_assets: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    game_data: Res<GameData>,
    time: Res<Time>,
) {
    for (entity, npc, command, move_mode, mut current_npc_model, skinned_mesh, transform) in
        query.iter_mut()
    {
        if let Some(current_npc_model) = current_npc_model.as_mut() {
            if current_npc_model.npc_id == npc.id {
                // Does not need new model, ignore
                continue;
            }

            // Despawn model parts
            for part_entity in current_npc_model.model_parts.iter() {
                commands.entity(*part_entity).despawn();
            }

            // Despawn model skeleton
            if let Some(skinned_mesh) = skinned_mesh {
                for bone_entity in skinned_mesh.joints.iter() {
                    commands.entity(*bone_entity).despawn();
                }
            }
        }

        if let Some((npc_model, skinned_mesh)) = model_loader.spawn_npc_model(
            &mut commands,
            &asset_server,
            &mut static_mesh_materials,
            &mut skinned_mesh_inverse_bindposes_assets,
            entity,
            npc.id,
        ) {
            let transform = if let Some(npc_data) = game_data.npcs.get_npc(npc.id) {
                transform.with_scale(Vec3::new(npc_data.scale, npc_data.scale, npc_data.scale))
            } else {
                *transform
            };

            if let (Some(command), Some(move_mode)) = (command, move_mode) {
                if let Some(motion) = get_command_motion(&npc_model, move_mode, command) {
                    commands.entity(entity).insert_bundle((
                        CommandNpcMotion {
                            command: command.command.clone(),
                            move_mode: *move_mode,
                        },
                        ActiveMotion::new(motion.clone(), time.seconds_since_startup()),
                    ));
                }
            }

            commands
                .entity(entity)
                .insert_bundle((npc_model, skinned_mesh, transform));
        } else {
            commands
                .entity(entity)
                .insert(NpcModel {
                    npc_id: npc.id,
                    model_parts: Vec::new(),
                    dummy_bone_offset: 0,
                    action_motions: Vec::new(),
                })
                .remove::<SkinnedMesh>();
        }
    }
}

#[derive(Component)]
pub struct NpcColliderRootBoneOffset {
    pub offset: Vec3,
}

pub fn npc_model_add_collider_system(
    mut commands: Commands,
    query_models: Query<(Entity, &NpcModel, &SkinnedMesh), Without<ColliderShapeComponent>>,
    mut query_collider_position: Query<(
        &SkinnedMesh,
        &NpcColliderRootBoneOffset,
        &mut ColliderPositionComponent,
    )>,
    query_aabb: Query<(&Aabb, Option<&SkinnedMesh>)>,
    query_global_transform: Query<&GlobalTransform>,
    inverse_bindposes: Res<Assets<SkinnedMeshInverseBindposes>>,
) {
    // Add colliders to NPC models without one
    for (entity, npc_model, skinned_mesh) in query_models.iter() {
        let mut min: Option<Vec3A> = None;
        let mut max: Option<Vec3A> = None;
        let mut all_parts_loaded = true;

        // Collect the AABB of skinned mesh parts
        for part_entity in npc_model.model_parts.iter() {
            if let Ok((aabb, skinned_mesh)) = query_aabb.get(*part_entity) {
                if skinned_mesh.is_some() {
                    min = Some(min.map_or_else(|| aabb.min(), |min| min.min(aabb.min())));
                    max = Some(max.map_or_else(|| aabb.max(), |max| max.max(aabb.max())));
                }
            } else {
                all_parts_loaded = false;
                break;
            }
        }

        let inverse_bindpose = inverse_bindposes.get(&skinned_mesh.inverse_bindposes);
        let root_bone_global_transform = query_global_transform.get(skinned_mesh.joints[0]).ok();
        if min.is_none()
            || max.is_none()
            || !all_parts_loaded
            || inverse_bindpose.is_none()
            || root_bone_global_transform.is_none()
        {
            continue;
        }
        let min = min.unwrap();
        let max = max.unwrap();
        let root_bone_global_transform = root_bone_global_transform.unwrap();
        let inverse_bindpose = inverse_bindpose.unwrap();
        let root_bone_local_transform = Transform::from_matrix(inverse_bindpose[0].inverse());

        let local_bound_center = 0.5 * (min + max);
        let half_extents = Vec3::from(0.5 * (max - min)) * root_bone_global_transform.scale;
        let root_bone_offset =
            Vec3::from(local_bound_center) - root_bone_local_transform.translation;

        commands
            .entity(entity)
            .insert_bundle(ColliderBundle {
                shape: ColliderShapeComponent(ColliderShape::cuboid(
                    half_extents.x,
                    half_extents.y,
                    half_extents.z,
                )),
                flags: ColliderFlagsComponent(ColliderFlags {
                    collision_groups: InteractionGroups::new(
                        COLLISION_GROUP_NPC,
                        COLLISION_FILTER_INSPECTABLE | COLLISION_FILTER_CLICKABLE,
                    ),
                    ..Default::default()
                }),
                position: ColliderPositionComponent(ColliderPosition(
                    (
                        root_bone_global_transform.translation + root_bone_offset,
                        root_bone_global_transform.rotation
                            * Quat::from_axis_angle(Vec3::Z, std::f32::consts::PI / 2.0),
                    )
                        .into(),
                )),
                ..Default::default()
            })
            .insert(NpcColliderRootBoneOffset {
                offset: root_bone_offset,
            });
    }

    // Update any existing collider's position
    for (skinned_mesh, root_bone_offset, mut collider_position) in
        query_collider_position.iter_mut()
    {
        if let Ok(root_bone_global_transform) = query_global_transform.get(skinned_mesh.joints[0]) {
            collider_position.translation =
                (root_bone_global_transform.translation + root_bone_offset.offset).into();
            collider_position.rotation = (root_bone_global_transform.rotation
                * Quat::from_axis_angle(Vec3::Z, std::f32::consts::PI / 2.0))
            .into();
        }
    }
}
