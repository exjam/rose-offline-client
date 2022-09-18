use bevy::{
    math::Vec3Swizzles,
    prelude::{Commands, Entity, Query, Res, Time},
};
use rose_game_common::components::{Destination, MoveSpeed};

use crate::components::{FacingDirection, Position};

pub fn update_position_system(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &MoveSpeed,
        &mut FacingDirection,
        &mut Position,
        &Destination,
    )>,
    time: Res<Time>,
) {
    for (entity, move_speed, mut facing_direction, mut position, destination) in query.iter_mut() {
        let direction = destination.position.xy() - position.xy();
        let distance_squared = direction.length_squared();

        if distance_squared == 0.0 {
            position.position = destination.position;
            commands.entity(entity).remove::<Destination>();
        } else {
            // Update rotation
            facing_direction.set_desired_vector(destination.position - position.position);

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
    }
}
