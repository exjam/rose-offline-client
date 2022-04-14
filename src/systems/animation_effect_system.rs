use bevy::{
    hierarchy::BuildChildren,
    math::Vec3,
    prelude::{
        AssetServer, Assets, Commands, ComputedVisibility, Entity, EventReader, GlobalTransform,
        Query, Res, ResMut, Transform, Visibility,
    },
    render::mesh::skinning::SkinnedMesh,
};
use rose_data::{AnimationEventFlags, SkillData, SkillType};
use rose_game_common::components::{MoveSpeed, Target};

use crate::{
    components::{CharacterModel, Command, CommandCastSkillTarget, NpcModel, Projectile},
    effect_loader::spawn_effect,
    events::AnimationFrameEvent,
    render::{EffectMeshMaterial, ParticleMaterial},
    resources::{ClientEntityList, GameData},
    VfsResource,
};

pub fn animation_effect_system(
    mut commands: Commands,
    mut animation_frame_events: EventReader<AnimationFrameEvent>,
    query_command: Query<&Command>,
    query_skeleton: Query<(&SkinnedMesh, Option<&CharacterModel>, Option<&NpcModel>)>,
    query_transform: Query<&GlobalTransform>,
    game_data: Res<GameData>,
    asset_server: Res<AssetServer>,
    client_entity_list: Res<ClientEntityList>,
    vfs_resource: Res<VfsResource>,
    mut effect_mesh_materials: ResMut<Assets<EffectMeshMaterial>>,
    mut particle_materials: ResMut<Assets<ParticleMaterial>>,
) {
    for event in animation_frame_events.iter() {
        if client_entity_list.player_entity == Some(event.entity) {
            log::trace!(target: "animation", "Player animation event flags: {:?}", event.flags);
        }

        if event
            .flags
            .contains(AnimationEventFlags::EFFECT_SKILL_ACTION)
        {
            if let Ok(Command::CastSkill(command_cast_skill)) = query_command.get(event.entity) {
                if let Some(skill_data) = game_data.skills.get_skill(command_cast_skill.skill_id) {
                    match skill_data.skill_type {
                        SkillType::TargetBound
                        | SkillType::TargetBoundDuration
                        | SkillType::TargetStateDuration
                        | SkillType::Resurrection => {
                            if let Some(CommandCastSkillTarget::Entity(target_entity)) =
                                command_cast_skill.skill_target
                            {
                                if let Ok(source_transform) = query_transform.get(event.entity) {
                                    if let Some(effect_data) = skill_data
                                        .bullet_effect_id
                                        .and_then(|id| game_data.effect_database.get_effect(id))
                                    {
                                        // TODO: effect_data.bullet_move_type;

                                        if let Some(effect_file_path) =
                                            effect_data.bullet_normal.and_then(|effect_file_id| {
                                                game_data
                                                    .effect_database
                                                    .get_effect_file(effect_file_id)
                                            })
                                        {
                                            if let Some(effect_entity) = spawn_effect(
                                                &vfs_resource.vfs,
                                                &mut commands,
                                                &asset_server,
                                                &mut particle_materials,
                                                &mut effect_mesh_materials,
                                                effect_file_path.into(),
                                                false,
                                            ) {
                                                commands
                                                    .spawn_bundle((
                                                        Projectile::new(
                                                            event.entity,
                                                            skill_data.hit_effect_file_id,
                                                        ),
                                                        Target::new(target_entity),
                                                        MoveSpeed::new(
                                                            effect_data.bullet_speed / 100.0,
                                                        ),
                                                        Transform::from_translation(
                                                            source_transform.translation
                                                                + Vec3::new(0.0, 0.5, 0.0),
                                                        ),
                                                        GlobalTransform::default(),
                                                        Visibility::default(),
                                                        ComputedVisibility::default(),
                                                    ))
                                                    .add_child(effect_entity);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        _ => log::warn!(
                            "Unimplemented EFFECT_SKILL_ACTION for skill type {:?}",
                            skill_data.skill_type
                        ),
                    }
                }
            }
        }

        if event
            .flags
            .contains(AnimationEventFlags::EFFECT_SKILL_CASTING_0)
        {
            if let Ok(Command::CastSkill(command_cast_skill)) = query_command.get(event.entity) {
                if let Some(skill_data) = game_data.skills.get_skill(command_cast_skill.skill_id) {
                    show_casting_effect(
                        &mut commands,
                        &asset_server,
                        &mut particle_materials,
                        &mut effect_mesh_materials,
                        &game_data,
                        &vfs_resource,
                        event.entity,
                        skill_data,
                        0,
                        &query_skeleton,
                    );
                }
            }
        }

        if event
            .flags
            .contains(AnimationEventFlags::EFFECT_SKILL_CASTING_1)
        {
            if let Ok(Command::CastSkill(command_cast_skill)) = query_command.get(event.entity) {
                if let Some(skill_data) = game_data.skills.get_skill(command_cast_skill.skill_id) {
                    show_casting_effect(
                        &mut commands,
                        &asset_server,
                        &mut particle_materials,
                        &mut effect_mesh_materials,
                        &game_data,
                        &vfs_resource,
                        event.entity,
                        skill_data,
                        1,
                        &query_skeleton,
                    );
                }
            }
        }

        if event
            .flags
            .contains(AnimationEventFlags::EFFECT_SKILL_CASTING_2)
        {
            if let Ok(Command::CastSkill(command_cast_skill)) = query_command.get(event.entity) {
                if let Some(skill_data) = game_data.skills.get_skill(command_cast_skill.skill_id) {
                    show_casting_effect(
                        &mut commands,
                        &asset_server,
                        &mut particle_materials,
                        &mut effect_mesh_materials,
                        &game_data,
                        &vfs_resource,
                        event.entity,
                        skill_data,
                        2,
                        &query_skeleton,
                    );
                }
            }
        }

        if event
            .flags
            .contains(AnimationEventFlags::EFFECT_SKILL_CASTING_3)
        {
            if let Ok(Command::CastSkill(command_cast_skill)) = query_command.get(event.entity) {
                if let Some(skill_data) = game_data.skills.get_skill(command_cast_skill.skill_id) {
                    show_casting_effect(
                        &mut commands,
                        &asset_server,
                        &mut particle_materials,
                        &mut effect_mesh_materials,
                        &game_data,
                        &vfs_resource,
                        event.entity,
                        skill_data,
                        3,
                        &query_skeleton,
                    );
                }
            }
        }
    }
}

fn show_casting_effect(
    commands: &mut Commands,
    asset_server: &AssetServer,
    particle_materials: &mut Assets<ParticleMaterial>,
    effect_mesh_materials: &mut Assets<EffectMeshMaterial>,
    game_data: &GameData,
    vfs_resource: &VfsResource,
    entity: Entity,
    skill_data: &SkillData,
    casting_effect_index: usize,
    query_skeleton: &Query<(&SkinnedMesh, Option<&CharacterModel>, Option<&NpcModel>)>,
) -> Option<()> {
    let casting_effect = skill_data
        .casting_effects
        .get(casting_effect_index)
        .and_then(|x| x.as_ref())?;
    let effect_path = game_data
        .effect_database
        .get_effect_file(casting_effect.effect_file_id)?;

    if let Some(effect_entity) = spawn_effect(
        &vfs_resource.vfs,
        commands,
        asset_server,
        particle_materials,
        effect_mesh_materials,
        effect_path.into(),
        false,
    ) {
        let mut link_entity = entity;

        if let Some(effect_dummy_bone_id) = casting_effect.effect_dummy_bone_id {
            if let Ok((skinned_mesh, character_model, npc_model)) = query_skeleton.get(entity) {
                if let Some(dummy_bone_offset) = character_model
                    .map(|character_model| character_model.dummy_bone_offset)
                    .or_else(|| npc_model.map(|npc_model| npc_model.dummy_bone_offset))
                {
                    if let Some(joint) = skinned_mesh
                        .joints
                        .get(dummy_bone_offset + effect_dummy_bone_id)
                    {
                        link_entity = *joint;
                    }
                }
            }
        }

        commands.entity(link_entity).add_child(effect_entity);
    }

    None
}
