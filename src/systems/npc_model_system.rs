use bevy::{
    core::Time,
    math::Vec3,
    prelude::{
        AssetServer, Assets, Changed, Commands, Component, Entity, Handle, Query, Res, ResMut,
        Transform, With,
    },
    render::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
};

use rose_game_common::components::{MoveMode, Npc};

use crate::{
    components::{ActiveMotion, Command, CommandData, NpcModel},
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
