use bevy::{
    prelude::{
        Commands, ComputedVisibility, EventReader, EventWriter, GlobalTransform, Query, Transform,
        Visibility,
    },
    render::mesh::skinning::SkinnedMesh,
};

use rose_game_common::components::{Destination, Target};

use crate::{
    components::{DummyBoneOffset, Projectile},
    events::{SpawnEffectData, SpawnEffectEvent, SpawnProjectileEvent, SpawnProjectileTarget},
};

pub fn spawn_projectile_system(
    mut commands: Commands,
    mut events: EventReader<SpawnProjectileEvent>,
    query_transform: Query<&GlobalTransform>,
    query_skeleton: Query<(&SkinnedMesh, &DummyBoneOffset)>,
    mut spawn_effect_events: EventWriter<SpawnEffectEvent>,
) {
    for event in events.iter() {
        let mut source_global_transform = None;

        if let Some(dummy_bone_id) = event.source_dummy_bone_id {
            if let Ok((skinned_mesh, dummy_bone_offset)) = query_skeleton.get(event.source) {
                if let Some(joint) = skinned_mesh
                    .joints
                    .get(dummy_bone_offset.index + dummy_bone_id)
                {
                    source_global_transform = query_transform.get(*joint).ok();
                }
            }
        }

        if source_global_transform.is_none() {
            source_global_transform = query_transform.get(event.source).ok();
        }

        if source_global_transform.is_none() {
            continue;
        }
        let source_global_transform = source_global_transform.unwrap();

        let mut entity_commands = commands.spawn_bundle((
            Projectile::new(
                event.source,
                event.source_skill_id,
                event.move_type,
                event.hit_effect_file_id,
            ),
            event.move_speed,
            Transform::from_translation(source_global_transform.translation),
            GlobalTransform::default(),
            Visibility::default(),
            ComputedVisibility::default(),
        ));

        match event.target {
            SpawnProjectileTarget::Entity(target_entity) => {
                entity_commands.insert(Target::new(target_entity));
            }
            SpawnProjectileTarget::Position(target_position) => {
                entity_commands.insert(Destination::new(target_position));
            }
        }

        if let Some(projectile_effect_file_id) = event.projectile_effect_file_id {
            spawn_effect_events.send(SpawnEffectEvent::OnEntity(
                entity_commands.id(),
                SpawnEffectData::with_file_id(projectile_effect_file_id),
            ));
        }
    }
}
