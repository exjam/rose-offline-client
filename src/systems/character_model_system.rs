use bevy::{
    core::Time,
    math::{Quat, Vec3, Vec3A},
    prelude::{
        AssetServer, Assets, Changed, Commands, Component, Entity, GlobalTransform, Handle, Or,
        Query, Res, ResMut, Transform, With, Without,
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
use rose_data::CharacterMotionAction;
use rose_game_common::components::{CharacterGender, CharacterInfo, Equipment, MoveMode};

use crate::{
    components::{
        ActiveMotion, CharacterModel, CharacterModelPart, Command, CommandData,
        COLLISION_FILTER_CLICKABLE, COLLISION_FILTER_INSPECTABLE, COLLISION_GROUP_CHARACTER,
    },
    model_loader::ModelLoader,
    render::StaticMeshMaterial,
    zmo_asset_loader::ZmoAsset,
};

#[derive(Component)]
pub struct CommandCharacterMotion {
    pub command: CommandData,
    pub gender: CharacterGender,
    pub move_mode: MoveMode,
    pub weapon_id: usize,
}

fn get_command_motion(
    character_model: &CharacterModel,
    move_mode: &MoveMode,
    command: &Command,
) -> Handle<ZmoAsset> {
    let action = match command.command {
        CommandData::Stop => CharacterMotionAction::Stop1,
        CommandData::Attack(_) => CharacterMotionAction::Attack,
        CommandData::Move(_) => match move_mode {
            MoveMode::Walk => CharacterMotionAction::Walk,
            MoveMode::Run => CharacterMotionAction::Run,
            _ => todo!("Character animation for driving cart"),
        },
    };

    character_model.action_motions[action].clone()
}

pub fn character_model_animation_system(
    mut commands: Commands,
    mut query_command: Query<
        (
            Entity,
            &CharacterModel,
            &Command,
            &MoveMode,
            Option<&CommandCharacterMotion>,
        ),
        With<SkinnedMesh>,
    >,
    time: Res<Time>,
) {
    for (entity, character_model, command, move_mode, command_npc_motion) in
        query_command.iter_mut()
    {
        if command_npc_motion.map_or(false, |x| {
            std::mem::discriminant(&x.command) == std::mem::discriminant(&command.command)
                && x.gender == character_model.gender
                && x.move_mode == *move_mode
                && x.weapon_id == character_model.model_parts[CharacterModelPart::Weapon].0
        }) {
            continue;
        }

        commands.entity(entity).insert_bundle((
            CommandCharacterMotion {
                command: command.command.clone(),
                gender: character_model.gender,
                move_mode: *move_mode,
                weapon_id: character_model.model_parts[CharacterModelPart::Weapon].0,
            },
            ActiveMotion::new(
                get_command_motion(character_model, move_mode, command),
                time.seconds_since_startup(),
            ),
        ));
    }
}

#[derive(Component)]
pub struct CharacterColliderRootBoneOffset {
    pub offset: Vec3,
}

pub fn character_model_add_collider_system(
    mut commands: Commands,
    query_models: Query<(Entity, &CharacterModel, &SkinnedMesh), Without<ColliderShapeComponent>>,
    mut query_collider_position: Query<(
        &SkinnedMesh,
        &CharacterColliderRootBoneOffset,
        &mut ColliderPositionComponent,
    )>,
    query_aabb: Query<&Aabb>,
    query_global_transform: Query<&GlobalTransform>,
    inverse_bindposes: Res<Assets<SkinnedMeshInverseBindposes>>,
) {
    // Add colliders to character models without one
    for (entity, character_model, skinned_mesh) in query_models.iter() {
        let mut min: Option<Vec3A> = None;
        let mut max: Option<Vec3A> = None;
        let mut all_parts_loaded = true;

        // Collect the AABB of Body, Hands, Feet
        for part_entity in character_model.model_parts[CharacterModelPart::Body]
            .1
            .iter()
            .chain(
                character_model.model_parts[CharacterModelPart::Hands]
                    .1
                    .iter(),
            )
            .chain(
                character_model.model_parts[CharacterModelPart::Feet]
                    .1
                    .iter(),
            )
        {
            if let Ok(aabb) = query_aabb.get(*part_entity) {
                min = Some(min.map_or_else(|| aabb.min(), |min| min.min(aabb.min())));
                max = Some(max.map_or_else(|| aabb.max(), |max| max.max(aabb.max())));
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
            || root_bone_global_transform.is_none()
            || inverse_bindpose.is_none()
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
                        COLLISION_GROUP_CHARACTER,
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
            .insert(CharacterColliderRootBoneOffset {
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

pub fn character_model_system(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &CharacterInfo,
            &Equipment,
            Option<&mut CharacterModel>,
            Option<&SkinnedMesh>,
        ),
        Or<(Changed<CharacterInfo>, Changed<Equipment>)>,
    >,
    asset_server: Res<AssetServer>,
    model_loader: Res<ModelLoader>,
    mut static_mesh_materials: ResMut<Assets<StaticMeshMaterial>>,
    mut skinned_mesh_inverse_bindposes_assets: ResMut<Assets<SkinnedMeshInverseBindposes>>,
) {
    for (entity, character_info, equipment, mut character_model, skinned_mesh) in query.iter_mut() {
        if let Some(character_model) = character_model.as_mut() {
            if character_model.gender == character_info.gender {
                model_loader.update_character_equipment(
                    &mut commands,
                    &asset_server,
                    &mut static_mesh_materials,
                    entity,
                    character_info,
                    equipment,
                    character_model,
                    skinned_mesh.as_ref().unwrap(),
                );
                continue;
            }

            // If character gender changed, we must destroy the previous model and create a new one
            for (_, (_, part_entities)) in character_model.model_parts.iter() {
                for part_entity in part_entities.iter() {
                    commands.entity(*part_entity).despawn();
                }
            }

            if let Some(skinned_mesh) = skinned_mesh {
                for joint in skinned_mesh.joints.iter() {
                    commands.entity(*joint).despawn();
                }
            }
        }

        let (character_model, skinned_mesh) = model_loader.spawn_character_model(
            &mut commands,
            &asset_server,
            &mut static_mesh_materials,
            &mut skinned_mesh_inverse_bindposes_assets,
            entity,
            character_info,
            equipment,
        );
        commands
            .entity(entity)
            .insert_bundle((character_model, skinned_mesh));
    }
}
