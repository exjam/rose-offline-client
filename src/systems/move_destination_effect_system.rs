use bevy::prelude::{
    AssetServer, Assets, Commands, ComputedVisibility, DespawnRecursiveExt, Entity, EventReader,
    GlobalTransform, Local, Res, ResMut, Transform, Visibility,
};
use rose_data::EffectFileId;

use crate::{
    effect_loader::spawn_effect,
    events::MoveDestinationEffectEvent,
    render::{EffectMeshMaterial, ParticleMaterial},
    resources::{GameData, VfsResource},
};

#[derive(Default)]
pub struct MoveDestinationEffectSystemState {
    pub last_effect_entity: Option<Entity>,
}

pub fn move_destination_effect_system(
    mut commands: Commands,
    mut state: Local<MoveDestinationEffectSystemState>,
    mut events: EventReader<MoveDestinationEffectEvent>,
    game_data: Res<GameData>,
    asset_server: Res<AssetServer>,
    vfs_resource: Res<VfsResource>,
    mut effect_mesh_materials: ResMut<Assets<EffectMeshMaterial>>,
    mut particle_materials: ResMut<Assets<ParticleMaterial>>,
) {
    for event in events.iter() {
        match event {
            MoveDestinationEffectEvent::Show { position } => {
                if let Some(last_effect_entity) = state.last_effect_entity.take() {
                    commands.entity(last_effect_entity).despawn_recursive();
                }

                if let Some(effect_file_path) = game_data
                    .effect_database
                    .get_effect_file(EffectFileId::new(296).unwrap())
                    .map(|x| x.into())
                {
                    let effect_entity = commands
                        .spawn((
                            Transform::from_translation(*position),
                            GlobalTransform::default(),
                            Visibility::default(),
                            ComputedVisibility::default(),
                        ))
                        .id();
                    state.last_effect_entity = Some(effect_entity);

                    spawn_effect(
                        &vfs_resource.vfs,
                        &mut commands,
                        &asset_server,
                        &mut particle_materials,
                        &mut effect_mesh_materials,
                        effect_file_path,
                        true,
                        Some(effect_entity),
                    );
                }
            }
            MoveDestinationEffectEvent::Hide => {
                if let Some(last_effect_entity) = state.last_effect_entity.take() {
                    commands.entity(last_effect_entity).despawn_recursive();
                }
            }
        }
    }
}
