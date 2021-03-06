use bevy::{
    hierarchy::{Children, DespawnRecursiveExt},
    prelude::{Commands, Entity, Query},
};

use crate::components::{Effect, EffectParticle, ParticleSequence};

pub fn effect_system(
    mut commands: Commands,
    query_effects: Query<(Entity, &Children, &Effect)>,
    query_children: Query<&Children>,
    query_particle_sequence: Query<(&EffectParticle, &ParticleSequence)>,
    // query_effect_mesh: Query<&EffectMesh>,
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
                    if let Ok((_, particle_sequence)) = query_particle_sequence.get(*child) {
                        if particle_sequence.finished && particle_sequence.particles.is_empty() {
                            children_finished += 1;
                        } else {
                            children_running += 1;
                        }
                    }

                    /*
                    if let Ok(_) = query_effect_mesh.get(*child) {
                        // TODO: Check if effect mesh is finished
                    }
                    */
                }
            }
        }

        if children_finished > 0 && children_running == 0 {
            commands.entity(effect_entity).despawn_recursive();
        }
    }
}
