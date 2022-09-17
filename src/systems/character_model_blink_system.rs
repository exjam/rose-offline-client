use bevy::prelude::{Assets, Commands, Handle, Query, Res, Time};
use rand::Rng;

use crate::{
    components::{CharacterBlinkTimer, CharacterModel, CharacterModelPart, Dead},
    render::ObjectMaterialClipFace,
    zms_asset_loader::ZmsMaterialNumFaces,
};

pub fn character_model_blink_system(
    mut commands: Commands,
    mut query_characters: Query<(&CharacterModel, &mut CharacterBlinkTimer, Option<&Dead>)>,
    query_material: Query<&Handle<ZmsMaterialNumFaces>>,
    material_assets: Res<Assets<ZmsMaterialNumFaces>>,
    time: Res<Time>,
) {
    for (character_model, mut blink_timer, dead) in query_characters.iter_mut() {
        let mut changed = false;

        if dead.is_none() {
            blink_timer.timer += time.delta_seconds();

            if blink_timer.is_open {
                if blink_timer.timer >= blink_timer.open_duration {
                    blink_timer.is_open = false;
                    blink_timer.timer -= blink_timer.open_duration;
                    blink_timer.closed_duration =
                        rand::thread_rng().gen_range(CharacterBlinkTimer::BLINK_CLOSED_DURATION);
                    changed = true;
                }
            } else if blink_timer.timer >= blink_timer.closed_duration {
                blink_timer.is_open = true;
                blink_timer.timer -= blink_timer.closed_duration;
                blink_timer.open_duration =
                    rand::thread_rng().gen_range(CharacterBlinkTimer::BLINK_OPEN_DURATION);
                changed = true;
            }
        } else {
            if blink_timer.is_open {
                blink_timer.is_open = false;

                // Set timer so the eyes open as soon as resurrected
                blink_timer.closed_duration = 0.0;
                blink_timer.timer = 0.0;
            }

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
