use bevy::{
    hierarchy::BuildChildren,
    prelude::{AssetServer, Assets, Commands, Entity, EventReader, Query, Res, ResMut},
    render::mesh::skinning::SkinnedMesh,
};
use rose_data::{AnimationEventFlags, SkillData};

use crate::{
    components::{CharacterModel, Command, NpcModel},
    effect_loader::spawn_effect,
    events::AnimationFrameEvent,
    render::{EffectMeshMaterial, ParticleMaterial},
    resources::GameData,
    VfsResource,
};

pub fn animation_effect_system(
    mut commands: Commands,
    mut animation_frame_events: EventReader<AnimationFrameEvent>,
    query_command: Query<&Command>,
    query_skeleton: Query<(&SkinnedMesh, Option<&CharacterModel>, Option<&NpcModel>)>,
    game_data: Res<GameData>,
    asset_server: Res<AssetServer>,
    vfs_resource: Res<VfsResource>,
    mut effect_mesh_materials: ResMut<Assets<EffectMeshMaterial>>,
    mut particle_materials: ResMut<Assets<ParticleMaterial>>,
) {
    for event in animation_frame_events.iter() {
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
        .get_effect(casting_effect.effect_id)?;

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
