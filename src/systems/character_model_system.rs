use bevy::{
    core::Time,
    prelude::{
        AssetServer, Assets, Changed, Commands, Component, Entity, Handle, Or, Query, Res, ResMut,
        With,
    },
    render::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
};

use rose_data::CharacterMotionAction;
use rose_game_common::components::{CharacterInfo, Equipment, MoveMode};

use crate::{
    components::{ActiveMotion, CharacterModel, CharacterModelPart, Command, CommandData},
    model_loader::ModelLoader,
    render::StaticMeshMaterial,
    zmo_asset_loader::ZmoAsset,
};

#[derive(Component)]
pub struct CommandCharacterMotion {
    pub command: CommandData,
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
        CommandData::Move(_) => match move_mode {
            MoveMode::Walk => CharacterMotionAction::Walk,
            MoveMode::Run => CharacterMotionAction::Run,
            _ => todo!("Character animation for driving cart"),
        },
    };

    character_model.action_motions[action].clone()
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
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
                && x.move_mode == *move_mode
                && x.weapon_id == character_model.model_parts[CharacterModelPart::Weapon].0
        }) {
            continue;
        }

        commands.entity(entity).insert_bundle((
            CommandCharacterMotion {
                command: command.command.clone(),
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

#[allow(clippy::type_complexity)]
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
        } else {
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
}
