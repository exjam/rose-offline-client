use std::ops::Range;

use bevy::{
    hierarchy::DespawnRecursiveExt,
    math::{Quat, Vec3, Vec3A},
    prelude::{
        AssetServer, Assets, BuildChildren, Changed, Commands, Component, Entity, GlobalTransform,
        Handle, Mesh, Or, Query, Res, ResMut, Time, Transform, With, Without,
    },
    render::{
        mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
        primitives::Aabb,
    },
};
use bevy_rapier3d::prelude::{Collider, CollisionGroups};

use rand::Rng;
use rose_game_common::components::{CharacterInfo, Equipment};

use crate::{
    components::{
        CharacterModel, CharacterModelPart, ColliderEntity, ColliderParent, DummyBoneOffset,
        ModelHeight, PersonalStore, PersonalStoreModel, PlayerCharacter,
        COLLISION_FILTER_CLICKABLE, COLLISION_FILTER_INSPECTABLE, COLLISION_GROUP_CHARACTER,
        COLLISION_GROUP_PHYSICS_TOY, COLLISION_GROUP_PLAYER,
    },
    model_loader::ModelLoader,
    render::{ObjectMaterial, ObjectMaterialClipFace},
    zms_asset_loader::ZmsMaterialNumFaces,
};

#[derive(Component)]
pub struct CharacterColliderRootBoneOffset {
    pub offset: Vec3,
}

const BLINK_CLOSED_DURATION: Range<f32> = 0.010..0.110;
const BLINK_OPEN_DURATION: Range<f32> = 0.100..3.000;

#[derive(Component)]
pub struct CharacterBlinkTimer {
    pub timer: f32,
    pub is_open: bool,
    pub closed_duration: f32,
    pub open_duration: f32,
}

impl CharacterBlinkTimer {
    pub fn new() -> Self {
        Self {
            timer: 0.0,
            is_open: false,
            closed_duration: rand::thread_rng().gen_range(BLINK_CLOSED_DURATION),
            open_duration: rand::thread_rng().gen_range(BLINK_OPEN_DURATION),
        }
    }
}

pub fn character_model_add_collider_system(
    mut commands: Commands,
    query_models: Query<
        (
            Entity,
            &CharacterModel,
            &SkinnedMesh,
            Option<&PlayerCharacter>,
        ),
        Without<ColliderEntity>,
    >,
    query_aabb: Query<Option<&Aabb>, With<Handle<Mesh>>>,
    inverse_bindposes: Res<Assets<SkinnedMeshInverseBindposes>>,
) {
    // Add colliders to character models without one
    for (entity, character_model, skinned_mesh, player_character) in query_models.iter() {
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
                if let Some(aabb) = aabb {
                    min = Some(min.map_or_else(|| aabb.min(), |min| min.min(aabb.min())));
                    max = Some(max.map_or_else(|| aabb.max(), |max| max.max(aabb.max())));
                } else {
                    all_parts_loaded = false;
                    break;
                }
            }
        }

        let inverse_bindpose = inverse_bindposes.get(&skinned_mesh.inverse_bindposes);
        let root_bone_entity = skinned_mesh.joints[0];
        if min.is_none() || max.is_none() || !all_parts_loaded || inverse_bindpose.is_none() {
            continue;
        }
        let min = Vec3::from(min.unwrap());
        let max = Vec3::from(max.unwrap());
        let root_bone_inverse_bindpose = Transform::from_matrix(inverse_bindpose.unwrap()[0]);

        let local_bound_center = 0.5 * (min + max);
        let half_extents = 0.5 * (max - min);
        let root_bone_offset = root_bone_inverse_bindpose.mul_vec3(local_bound_center);

        let collider_entity = commands
            .spawn_bundle((
                ColliderParent::new(entity),
                Collider::cuboid(half_extents.x, half_extents.y, half_extents.z),
                CollisionGroups::new(
                    if player_character.is_some() {
                        COLLISION_GROUP_PLAYER
                    } else {
                        COLLISION_GROUP_CHARACTER
                    },
                    COLLISION_FILTER_INSPECTABLE
                        | COLLISION_FILTER_CLICKABLE
                        | COLLISION_GROUP_PHYSICS_TOY,
                ),
                Transform::from_translation(root_bone_offset)
                    .with_rotation(Quat::from_axis_angle(Vec3::Z, std::f32::consts::PI / 2.0)),
                GlobalTransform::default(),
            ))
            .id();

        commands.entity(root_bone_entity).add_child(collider_entity);

        commands.entity(entity).insert_bundle((
            ColliderEntity::new(collider_entity),
            ModelHeight::new(half_extents.y * 2.0),
        ));
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
            Option<&DummyBoneOffset>,
            Option<&SkinnedMesh>,
            Option<&PersonalStore>,
            Option<&PersonalStoreModel>,
        ),
        Or<(
            Changed<CharacterInfo>,
            Changed<Equipment>,
            Changed<PersonalStore>,
        )>,
    >,
    asset_server: Res<AssetServer>,
    model_loader: Res<ModelLoader>,
    mut object_materials: ResMut<Assets<ObjectMaterial>>,
    mut skinned_mesh_inverse_bindposes_assets: ResMut<Assets<SkinnedMeshInverseBindposes>>,
) {
    for (
        entity,
        character_info,
        equipment,
        mut character_model,
        dummy_bone_offset,
        skinned_mesh,
        personal_store,
        personal_store_model,
    ) in query.iter_mut()
    {
        if let Some(character_model) = character_model.as_mut() {
            // If gender has not changed, we can just update our equipment models
            if personal_store.is_none() && character_model.gender == character_info.gender {
                model_loader.update_character_equipment(
                    &mut commands,
                    &asset_server,
                    &mut object_materials,
                    entity,
                    character_info,
                    equipment,
                    character_model,
                    dummy_bone_offset.as_ref().unwrap(),
                    skinned_mesh.as_ref().unwrap(),
                );
                continue;
            }

            // Destroy the previous model
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

            if personal_store.is_some() {
                commands
                    .entity(entity)
                    .remove::<CharacterModel>()
                    .remove::<SkinnedMesh>();
            }
        }

        if let Some(personal_store) = personal_store {
            if let Some(personal_store_model) = personal_store_model {
                // If the skin has changed, despawn it and spawn a new one
                if personal_store_model.skin == personal_store.skin {
                    continue;
                }

                commands
                    .entity(personal_store_model.model)
                    .despawn_recursive();
            }

            // Spawn new personal store model
            let personal_store_model = model_loader.spawn_personal_store_model(
                &mut commands,
                &asset_server,
                &mut object_materials,
                entity,
                personal_store.skin,
            );
            commands.entity(entity).insert(personal_store_model);
        } else {
            if let Some(personal_store_model) = personal_store_model {
                // Despawn personal store model
                commands
                    .entity(personal_store_model.model)
                    .despawn_recursive();
                commands.entity(entity).remove::<PersonalStoreModel>();
            }

            // Spawn new character model
            let (character_model, skinned_mesh, dummy_bone_offset) = model_loader
                .spawn_character_model(
                    &mut commands,
                    &asset_server,
                    &mut object_materials,
                    &mut skinned_mesh_inverse_bindposes_assets,
                    entity,
                    character_info,
                    equipment,
                );
            commands.entity(entity).insert_bundle((
                character_model,
                skinned_mesh,
                dummy_bone_offset,
                CharacterBlinkTimer::new(),
            ));
        }
    }
}

pub fn character_model_blink_system(
    mut commands: Commands,
    mut query_characters: Query<(&CharacterModel, &mut CharacterBlinkTimer)>,
    query_material: Query<&Handle<ZmsMaterialNumFaces>>,
    material_assets: Res<Assets<ZmsMaterialNumFaces>>,
    time: Res<Time>,
) {
    for (character_model, mut blink_timer) in query_characters.iter_mut() {
        let mut changed = false;
        blink_timer.timer += time.delta_seconds();

        if blink_timer.is_open {
            if blink_timer.timer >= blink_timer.open_duration {
                blink_timer.is_open = false;
                blink_timer.timer -= blink_timer.open_duration;
                blink_timer.closed_duration = rand::thread_rng().gen_range(BLINK_CLOSED_DURATION);
                changed = true;
            }
        } else if blink_timer.timer >= blink_timer.closed_duration {
            blink_timer.is_open = true;
            blink_timer.timer -= blink_timer.closed_duration;
            blink_timer.open_duration = rand::thread_rng().gen_range(BLINK_OPEN_DURATION);
            changed = true;
        }

        if changed {
            for face_model_entity in character_model.model_parts[CharacterModelPart::CharacterFace]
                .1
                .iter()
            {
                if let Ok(face_mesh_handle) = query_material.get(*face_model_entity) {
                    if let Some(face_mesh) = material_assets.get(face_mesh_handle) {
                        if let Some(num_clip_faces) = face_mesh.material_num_faces.last() {
                            if blink_timer.is_open {
                                commands
                                    .entity(*face_model_entity)
                                    .insert(ObjectMaterialClipFace::First(*num_clip_faces as u32));
                            } else {
                                commands
                                    .entity(*face_model_entity)
                                    .insert(ObjectMaterialClipFace::Last(*num_clip_faces as u32));
                            }
                        }
                    }
                }
            }
        }
    }
}
