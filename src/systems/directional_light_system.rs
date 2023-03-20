use bevy::prelude::{DirectionalLight, GlobalTransform, Query, Transform, With};

use crate::components::PlayerCharacter;

const PROJECTION_HALF_SIZE: f32 = 50.0;
const PROJECTION_HALF_DEPTH: f32 = 100.0;

pub fn directional_light_system(
    query_player: Query<&GlobalTransform, With<PlayerCharacter>>,
    mut query_light: Query<&mut Transform, With<DirectionalLight>>,
) {
    if let Ok(player_transform) = query_player.get_single() {
        if let Ok(mut light_transform) = query_light.get_single_mut() {
            light_transform = light_transform.with_translation(player_transform.translation());
        }
    }
}
