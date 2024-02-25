use bevy::{
    prelude::{Color, DirectionalLight, Gizmos, Local, Query, Res, Vec3, With},
    render::primitives::{CascadesFrusta, Frustum, HalfSpace},
};

use crate::resources::DebugRenderConfig;

fn calculate_frustum_corner(plane1: &HalfSpace, plane2: &HalfSpace, plane3: &HalfSpace) -> Vec3 {
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
                    &frustum.half_spaces[i],
                    &frustum.half_spaces[j],
                    &frustum.half_spaces[k],
                );
                index += 1;
            }
        }
    }

    corners
}

pub fn debug_render_directional_light_system(
    debug_render_config: Res<DebugRenderConfig>,
    query_cascades_frustum: Query<&CascadesFrusta, With<DirectionalLight>>,
    mut last_cascades_frusta: Local<CascadesFrusta>,
    mut gizmos: Gizmos,
) {
    if !debug_render_config.directional_light_frustum {
        return;
    }

    if !debug_render_config.directional_light_frustum_freeze {
        if let Ok(cascades_frusta) = query_cascades_frustum.get_single() {
            last_cascades_frusta.frusta = cascades_frusta.frusta.clone();
        }
    }

    for (_, frusta) in last_cascades_frusta.frusta.iter() {
        for frustum in frusta.iter() {
            let corners = calculate_frustum_corners(frustum);

            gizmos.linestrip(
                [corners[0], corners[2], corners[6], corners[4], corners[0]],
                Color::YELLOW_GREEN,
            );
            gizmos.linestrip(
                [corners[1], corners[3], corners[7], corners[5], corners[1]],
                Color::YELLOW_GREEN,
            );

            gizmos.line(corners[0], corners[1], Color::YELLOW_GREEN);
            gizmos.line(corners[2], corners[3], Color::YELLOW_GREEN);
            gizmos.line(corners[4], corners[5], Color::YELLOW_GREEN);
            gizmos.line(corners[6], corners[7], Color::YELLOW_GREEN);
        }
    }
}
