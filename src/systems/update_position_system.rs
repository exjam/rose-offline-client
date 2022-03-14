use bevy::{
    math::{Quat, Vec3},
    prelude::{Commands, Entity, Query, Transform},
};
use rose_game_common::components::{Destination, Position};

pub fn update_position_system(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Position, &Destination, &mut Transform)>,
) {
    for (entity, mut position, destination, mut transform) in query.iter_mut() {
        let direction = destination.position.xy() - position.position.xy();
        let distance_squared = direction.magnitude_squared();

        if distance_squared == 0.0 {
            position.position = destination.position;
            commands.entity(entity).remove::<Destination>();
        } else {
            // Update rotation
            let dx = destination.position.x - position.position.x;
            let dy = destination.position.y - position.position.y;
            transform.rotation =
                Quat::from_axis_angle(Vec3::Y, dy.atan2(dx) + std::f32::consts::PI / 2.0);

            // Move to position
            let move_vector = direction.normalize() * 300.0 * (1.0 / 60.0);
            if move_vector.magnitude_squared() >= distance_squared {
                position.position = destination.position;
                commands.entity(entity).remove::<Destination>();
            } else {
                position.position.x += move_vector.x;
                position.position.y += move_vector.y;
            }
        }

        transform.translation.x = position.position.x / 100.0;
        transform.translation.z = -position.position.y / 100.0;
    }
}
