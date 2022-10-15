use bevy::prelude::{
    DirectionalLight, GlobalTransform, Mat4, OrthographicProjection, Query, Vec3, With,
};

use crate::components::PlayerCharacter;

const PROJECTION_HALF_SIZE: f32 = 50.0;
const PROJECTION_HALF_DEPTH: f32 = 100.0;

pub fn directional_light_system(
    query_player: Query<&GlobalTransform, With<PlayerCharacter>>,
    mut query_light: Query<(&GlobalTransform, &mut DirectionalLight)>,
) {
    if let Ok(player_transform) = query_player.get_single() {
        if let Ok((light_transform, mut directional_light)) = query_light.get_single_mut() {
            let light_direction = light_transform.forward();
            let lookat_position = player_transform.translation();

            let view = Mat4::look_at_rh(Vec3::ZERO, light_direction, Vec3::Y);
            let projected = view.mul_vec4(lookat_position.extend(1.0));

            directional_light.shadow_projection = OrthographicProjection {
                left: projected.x - PROJECTION_HALF_SIZE,
                right: projected.x + PROJECTION_HALF_SIZE,
                bottom: projected.y + PROJECTION_HALF_SIZE,
                top: projected.y - PROJECTION_HALF_SIZE,
                near: -projected.z - PROJECTION_HALF_DEPTH,
                far: -projected.z + PROJECTION_HALF_DEPTH,
                ..Default::default()
            };
        }
    }
}
