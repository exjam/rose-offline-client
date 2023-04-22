use bevy::{
    math::Vec3Swizzles,
    prelude::{Query, Res, Time},
};

use rose_game_common::components::MoveSpeed;

use crate::components::{Command, CommandMove, FacingDirection, Position};

pub fn update_position_system(
    mut query: Query<(&Command, &MoveSpeed, &mut FacingDirection, &mut Position)>,
    time: Res<Time>,
) {
    for (command, move_speed, mut facing_direction, mut position) in query.iter_mut() {
        let Command::Move(CommandMove {
            destination,
            ..
        }) = *command else {
            continue;
        };

        let direction = destination.xy() - position.xy();
        let distance_squared = direction.length_squared();

        if distance_squared == 0.0 {
            position.position = destination;
        } else {
            // Update rotation
            facing_direction.set_desired_vector(destination - position.position);

            // Move to position
            let move_vector = direction.normalize() * move_speed.speed * time.delta_seconds();
            if move_vector.length_squared() >= distance_squared {
                position.position = destination;
            } else {
                position.x += move_vector.x;
                position.y += move_vector.y;
            }
        }
    }
}
