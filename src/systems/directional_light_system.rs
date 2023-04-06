use bevy::{
    pbr::{Cascade, Cascades, DirectionalLightShadowMap},
    prelude::{Camera, DirectionalLight, Entity, GlobalTransform, Mat4, Query, Res, Vec3, With},
};

use crate::components::PlayerCharacter;

const PROJECTION_HALF_SIZE: f32 = 40.0;
const PROJECTION_HALF_DEPTH: f32 = 100.0;

pub fn directional_light_system(
    query_player: Query<&GlobalTransform, With<PlayerCharacter>>,
    mut query_light: Query<(&GlobalTransform, &mut Cascades), With<DirectionalLight>>,
    views: Query<(Entity, &GlobalTransform), With<Camera>>,
    shadow_map: Res<DirectionalLightShadowMap>,
) {
    let lookat_position = if let Ok(player_transform) = query_player.get_single() {
        player_transform.translation()
    } else if let Ok((_, camera_transform)) = views.get_single() {
        camera_transform.translation()
    } else {
        return;
    };

    if let Ok((light_transform, mut cascades)) = query_light.get_single_mut() {
        let light_direction = light_transform.forward();
        let view = Mat4::look_at_rh(Vec3::ZERO, light_direction, Vec3::Y);
        let projected = view.mul_vec4(lookat_position.extend(1.0));

        let projection = Mat4::orthographic_rh(
            projected.x - PROJECTION_HALF_SIZE,
            projected.x + PROJECTION_HALF_SIZE,
            projected.y + PROJECTION_HALF_SIZE,
            projected.y - PROJECTION_HALF_SIZE,
            -projected.z + PROJECTION_HALF_DEPTH,
            -projected.z - PROJECTION_HALF_DEPTH,
        );

        let view_transform = light_transform.compute_matrix();
        let view_projection = projection * view_transform.inverse();

        cascades.cascades.clear();
        for (view_entity, _) in views.iter() {
            cascades.cascades.insert(
                view_entity,
                vec![Cascade {
                    view_transform,
                    projection,
                    view_projection,
                    texel_size: (PROJECTION_HALF_SIZE * 2.0) / (shadow_map.size as f32),
                }],
            );
        }
    }
}
