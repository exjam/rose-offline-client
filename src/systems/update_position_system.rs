use bevy::{
    math::{Quat, Vec3, Vec3Swizzles},
    prelude::{Commands, Entity, Query, Res, Time, Transform},
};
use rose_game_common::components::{Destination, MoveSpeed};

use crate::components::Position;

pub fn update_position_system(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &MoveSpeed,
        &mut Position,
        &Destination,
        &mut Transform,
    )>,
    time: Res<Time>,
) {
    for (entity, move_speed, mut position, destination, mut transform) in query.iter_mut() {
        let direction = destination.position.xy() - position.xy();
        let distance_squared = direction.length_squared();

        if distance_squared == 0.0 {
            position.position = destination.position;
            commands.entity(entity).remove::<Destination>();
        } else {
            // Update rotation
            let dx = destination.position.x - position.x;
            let dy = destination.position.y - position.y;
            transform.rotation =
                Quat::from_axis_angle(Vec3::Y, dy.atan2(dx) + std::f32::consts::PI / 2.0);

            // Move to position
            let move_vector = direction.normalize() * move_speed.speed * time.delta_seconds();
            if move_vector.length_squared() >= distance_squared {
                position.position = destination.position;
                commands.entity(entity).remove::<Destination>();
            } else {
                position.x += move_vector.x;
                position.y += move_vector.y;
            }
        }

        transform.translation.x = position.x / 100.0;
        transform.translation.z = -position.y / 100.0;
    }
}
