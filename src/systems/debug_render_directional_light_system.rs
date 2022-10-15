use bevy::{
    prelude::{DirectionalLight, Query, Res, ResMut, Vec3, With},
    render::primitives::{Frustum, Plane},
};

use crate::resources::{DebugRenderConfig, DebugRenderDirectionalLightData};

fn calculate_frustum_corner(plane1: &Plane, plane2: &Plane, plane3: &Plane) -> Vec3 {
    let denominator = plane1.normal().dot(plane2.normal().cross(plane3.normal()));
    if denominator.abs() > 0.0 {
        let mut nominator = -plane1.d() * plane2.normal().cross(plane3.normal());
        nominator -= plane2.d() * plane3.normal().cross(plane1.normal());
        nominator -= plane3.d() * plane1.normal().cross(plane2.normal());
        nominator *= 1.0 / denominator;
        nominator.into()
    } else {
        Vec3::ZERO
    }
}

fn calculate_frustum_corners(frustum: &Frustum) -> [Vec3; 8] {
    let mut corners: [Vec3; 8] = Default::default();
    let mut index = 0;

    for i in 0..2 {
        for j in 2..4 {
            for k in 4..6 {
                corners[index] = calculate_frustum_corner(
                    &frustum.planes[i],
                    &frustum.planes[j],
                    &frustum.planes[k],
                );
                index += 1;
            }
        }
    }

    corners
}

pub fn debug_render_directional_light_system(
    debug_render_config: Res<DebugRenderConfig>,
    query_light: Query<&Frustum, With<DirectionalLight>>,
    mut render_data: ResMut<DebugRenderDirectionalLightData>,
) {
    if !debug_render_config.directional_light_frustum {
        return;
    }

    if let Ok(frustum) = query_light.get_single() {
        let corners = calculate_frustum_corners(frustum);
        render_data.frustum.vertices.clear();
        render_data.frustum.vertices.push(corners[0]);
        render_data.frustum.vertices.push(corners[2]);
        render_data.frustum.vertices.push(corners[6]);
        render_data.frustum.vertices.push(corners[4]);
        render_data.frustum.vertices.push(corners[0]);
        render_data.frustum.vertices.push(Vec3::NAN);
        render_data.frustum.vertices.push(corners[1]);
        render_data.frustum.vertices.push(corners[3]);
        render_data.frustum.vertices.push(corners[7]);
        render_data.frustum.vertices.push(corners[5]);
        render_data.frustum.vertices.push(corners[1]);
        render_data.frustum.vertices.push(Vec3::NAN);
        render_data.frustum.vertices.push(corners[0]);
        render_data.frustum.vertices.push(corners[1]);
        render_data.frustum.vertices.push(Vec3::NAN);
        render_data.frustum.vertices.push(corners[2]);
        render_data.frustum.vertices.push(corners[3]);
        render_data.frustum.vertices.push(Vec3::NAN);
        render_data.frustum.vertices.push(corners[4]);
        render_data.frustum.vertices.push(corners[5]);
        render_data.frustum.vertices.push(Vec3::NAN);
        render_data.frustum.vertices.push(corners[6]);
        render_data.frustum.vertices.push(corners[7]);
    }
}
