use bevy::{
    hierarchy::Parent,
    prelude::{Color, Gizmos, GlobalTransform, Query, Res},
    render::mesh::skinning::SkinnedMesh,
};

use crate::resources::DebugRenderConfig;

pub fn debug_render_skeleton_system(
    debug_render_config: Res<DebugRenderConfig>,
    query_skeleton: Query<&SkinnedMesh>,
    query_bone: Query<(&GlobalTransform, Option<&Parent>)>,
    mut gizmos: Gizmos,
) {
    if !debug_render_config.skeleton && !debug_render_config.bone_up {
        return;
    }

    for skinned_mesh in query_skeleton.iter() {
        for bone_entity in skinned_mesh.joints.iter() {
            if let Ok((transform, parent)) = query_bone.get(*bone_entity) {
                let (_, rotation, translation) = transform.to_scale_rotation_translation();

                if debug_render_config.skeleton {
                    if let Some((parent_transform, _)) =
                        parent.and_then(|x| query_bone.get(x.get()).ok())
                    {
                        gizmos.line_gradient(
                            translation,
                            parent_transform.translation(),
                            Color::WHITE,
                            Color::GRAY,
                        );
                    }
                }

                if debug_render_config.bone_up {
                    let start = translation;
                    let end = translation + rotation.mul_vec3([0.0, 0.0, -0.2].into());
                    gizmos.line_gradient(start, end, Color::PINK, Color::PURPLE);
                }
            }
        }
    }
}
