use bevy::{
    hierarchy::{Children, DespawnRecursiveExt},
    prelude::{Commands, Entity, Query},
};

use crate::{
    animation::{MeshAnimation, TransformAnimation},
    components::{Effect, EffectMesh, EffectParticle, ParticleSequence},
};

pub fn effect_system(
    mut commands: Commands,
    query_effects: Query<(Entity, &Children, &Effect)>,
    query_children: Query<&Children>,
    query_particle_sequence: Query<(
        &EffectParticle,
        &ParticleSequence,
        Option<&TransformAnimation>,
    )>,
    query_effect_mesh: Query<(&EffectMesh, Option<&MeshAnimation>)>,
) {
    for (effect_entity, effect_children, effect) in query_effects.iter() {
        let mut children_finished = 0;
        let mut children_running = 0;

        if effect.manual_despawn {
            continue;
        }

        for child in effect_children.iter() {
            if let Ok(children) = query_children.get(*child) {
                for child in children.iter() {
                    if let Ok((_, particle_sequence, transform_animation)) =
                        query_particle_sequence.get(*child)
                    {
                        let particle_sequence_completed =
                            particle_sequence.finished && particle_sequence.particles.is_empty();
                        let transform_animation_completed = transform_animation
                            .map_or(true, |transform_animation| transform_animation.completed());

                        if particle_sequence_completed || transform_animation_completed {
                            children_finished += 1;
                        } else {
                            children_running += 1;
                        }
                    }

                    if let Ok((_, mesh_animation)) = query_effect_mesh.get(*child) {
                        if mesh_animation.map_or(true, |mesh_animation| mesh_animation.completed())
                        {
                            children_finished += 1;
                        } else {
                            children_running += 1;
                        }
                    }
                }
            }
        }

        if children_finished > 0 && children_running == 0 {
            commands.entity(effect_entity).despawn_recursive();
        }
    }
}
