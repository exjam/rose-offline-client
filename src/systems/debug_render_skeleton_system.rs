use bevy::{
    hierarchy::Parent,
    math::Vec3,
    prelude::{GlobalTransform, Query, Res, ResMut},
    render::mesh::skinning::SkinnedMesh,
};

use crate::resources::{DebugRenderConfig, DebugRenderSkeletonData};

pub fn debug_render_skeleton_system(
    debug_render_config: Res<DebugRenderConfig>,
    query_skeleton: Query<&SkinnedMesh>,
    query_bone: Query<(&GlobalTransform, Option<&Parent>)>,
    mut render_data: ResMut<DebugRenderSkeletonData>,
) {
    if !debug_render_config.skeleton && !debug_render_config.bone_up {
        return;
    }

    let render_data = &mut *render_data;
    let skeleton_vertices = &mut render_data.skeleton.vertices;
    let bone_up_vertices = &mut render_data.bone_up.vertices;

    for skinned_mesh in query_skeleton.iter() {
        for bone_entity in skinned_mesh.joints.iter() {
            if let Ok((transform, parent)) = query_bone.get(*bone_entity) {
                if debug_render_config.skeleton {
                    if let Some((parent_transform, _)) =
                        parent.and_then(|x| query_bone.get(x.0).ok())
                    {
                        skeleton_vertices.push(transform.translation);
                        skeleton_vertices.push(parent_transform.translation);
                        skeleton_vertices.push(Vec3::new(f32::NAN, f32::NAN, f32::NAN));
                    }
                }

                if debug_render_config.bone_up {
                    // Our bones seem to be in -Z up space
                    bone_up_vertices.push(transform.translation);
                    bone_up_vertices.push(
                        transform.translation
                            + transform.rotation.mul_vec3([0.0, 0.0, -0.2].into()),
                    );
                    bone_up_vertices.push(Vec3::new(f32::NAN, f32::NAN, f32::NAN));
                }
            }
        }
    }
}
