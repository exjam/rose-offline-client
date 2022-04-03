use bevy::{
    hierarchy::BuildChildren,
    prelude::{
        AssetServer, Assets, Commands, Entity, EventReader, EventWriter, Query, Res, ResMut, With,
    },
};

use crate::{
    components::PlayerCharacter,
    effect_loader::spawn_effect,
    events::{ChatboxEvent, PlayerCharacterEvent},
    render::{EffectMeshMaterial, ParticleMaterial},
    VfsResource,
};

pub fn player_character_event_system(
    mut commands: Commands,
    mut player_character_events: EventReader<PlayerCharacterEvent>,
    mut chatbox_events: EventWriter<ChatboxEvent>,
    query_player: Query<Entity, With<PlayerCharacter>>,
    asset_server: Res<AssetServer>,
    vfs_resource: Res<VfsResource>,
    mut effect_mesh_materials: ResMut<Assets<EffectMeshMaterial>>,
    mut particle_materials: ResMut<Assets<ParticleMaterial>>,
) {
    let player_entity = query_player.single();

    for event in player_character_events.iter() {
        match event {
            PlayerCharacterEvent::LevelUp(level) => {
                chatbox_events.send(ChatboxEvent::System(format!(
                    "Congratulations! You are now level {}!",
                    level
                )));

                if let Some(effect_entity) = spawn_effect(
                    &vfs_resource.vfs,
                    &mut commands,
                    &asset_server,
                    &mut particle_materials,
                    &mut effect_mesh_materials,
                    "3DDATA/EFFECT/LEVELUP_01.EFT".into(),
                ) {
                    commands.entity(player_entity).add_child(effect_entity);
                }
            }
        }
    }
}
