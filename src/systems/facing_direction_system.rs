use bevy::{
    prelude::{Quat, Query, Res, Transform, Vec3},
    time::Time,
};

use crate::components::FacingDirection;

const ROTATE_ANGLE_PER_SECOND: f32 = 3.0 * std::f32::consts::TAU;

pub fn facing_direction_system(
    mut query: Query<(&mut FacingDirection, &mut Transform)>,
    time: Res<Time>,
) {
    for (mut facing_direction, mut transform) in query.iter_mut() {
        let mut diff = facing_direction.desired - facing_direction.actual;
        if diff.abs() < 0.001 {
            continue;
        }

        if diff > std::f32::consts::PI {
            diff = -(std::f32::consts::TAU - diff);
        } else if diff < -std::f32::consts::PI {
            diff += std::f32::consts::TAU;
        }

        let mut rotate_amount = time.delta_seconds() * ROTATE_ANGLE_PER_SECOND;
        let x = (diff.abs() - rotate_amount).abs();
        let t = ((x * x) / (std::f32::consts::FRAC_PI_2 * std::f32::consts::FRAC_PI_2))
            .min(1.0)
            .max(0.3);
        rotate_amount *= t;

        if rotate_amount >= diff.abs() {
            facing_direction.actual = facing_direction.desired;
        } else if diff < 0.0 {
            facing_direction.actual -= rotate_amount;
        } else {
            facing_direction.actual += rotate_amount;
        }

        transform.rotation = Quat::from_axis_angle(
            Vec3::Y,
            facing_direction.actual - std::f32::consts::PI / 2.0,
        );
    }
}
