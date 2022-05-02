use bevy::{
    hierarchy::Parent,
    prelude::{Color, GlobalTransform, Query, Res, ResMut},
    render::mesh::skinning::SkinnedMesh,
};
use bevy_prototype_debug_lines::DebugLines;

use crate::resources::DebugRenderConfig;

pub fn debug_render_skeleton_system(
    debug_render_config: Res<DebugRenderConfig>,
    query_skeleton: Query<&SkinnedMesh>,
    query_bone: Query<(&GlobalTransform, Option<&Parent>)>,
    mut lines: ResMut<DebugLines>,
) {
    if !debug_render_config.skeleton && !debug_render_config.bone_up {
        return;
    }

    for skinned_mesh in query_skeleton.iter() {
        for bone_entity in skinned_mesh.joints.iter() {
            if let Ok((transform, parent)) = query_bone.get(*bone_entity) {
                if let Some((parent_transform, _)) = parent.and_then(|x| query_bone.get(x.0).ok()) {
                    if debug_render_config.skeleton {
                        lines.line_colored(
                            transform.translation,
                            parent_transform.translation,
                            0.0,
                            Color::PINK,
                        );
                    }

                    if debug_render_config.bone_up {
                        lines.line_colored(
                            transform.translation,
                            transform.translation
                                + transform.rotation.mul_vec3([0.0, 0.2, 0.0].into()),
                            0.0,
                            Color::GREEN,
                        );
                    }
                }
            }
        }
    }
}
