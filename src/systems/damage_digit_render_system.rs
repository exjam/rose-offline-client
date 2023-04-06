use bevy::{
    hierarchy::DespawnRecursiveExt,
    math::{Vec3Swizzles, Vec4},
    prelude::{Commands, Entity, GlobalTransform, Query},
};

use crate::{
    animation::TransformAnimation, components::DamageDigits, render::DamageDigitRenderData,
};

pub fn damage_digit_render_system(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &GlobalTransform,
        &TransformAnimation,
        &DamageDigits,
        &mut DamageDigitRenderData,
    )>,
) {
    for (entity, global_transform, animation, damage_digits, mut damage_digit_render_data) in
        query.iter_mut()
    {
        damage_digit_render_data.clear();

        if animation.completed() {
            // Animation completed, despawn
            commands.entity(entity).despawn_recursive();
            continue;
        }

        let (scale, _, translation) = global_transform.to_scale_rotation_translation();
        if damage_digits.damage == 0 {
            // Miss, split over 4 digits
            for digit in 0..4 {
                damage_digit_render_data.add(
                    translation,
                    -1.5 + digit as f32,
                    0.4 * scale.xy(),
                    Vec4::new(digit as f32 / 4.0, 0.0, (digit + 1) as f32 / 4.0, 1.0),
                );
            }
        } else {
            // First count the number of digits
            let mut damage = damage_digits.damage;
            let mut digit_count = 0;
            while damage > 0 {
                digit_count += 1;
                damage /= 10;
            }

            // Add digits to render data
            let number_offset = (digit_count - 1) as f32 / 2.0;
            let mut digit_offset = 0.0;
            let mut damage = damage_digits.damage;
            while damage > 0 {
                let digit = damage % 10;
                damage_digit_render_data.add(
                    translation,
                    number_offset - digit_offset,
                    0.4 * scale.xy(),
                    Vec4::new(digit as f32 / 10.0, 0.0, (digit + 1) as f32 / 10.0, 1.0),
                );
                digit_offset += 1.0;
                damage /= 10;
            }
        }
    }
}
