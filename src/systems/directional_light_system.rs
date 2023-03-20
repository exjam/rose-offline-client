use bevy::prelude::{DirectionalLight, GlobalTransform, Query, Transform, With};

use crate::components::PlayerCharacter;

pub fn directional_light_system(
    query_player: Query<&GlobalTransform, With<PlayerCharacter>>,
    mut query_light: Query<&mut Transform, With<DirectionalLight>>,
) {
    if let Ok(player_transform) = query_player.get_single() {
        if let Ok(mut light_transform) = query_light.get_single_mut() {
            light_transform.translation = player_transform.translation();
        }
    }
}
