use bevy::prelude::{Commands, Entity, Query, Res, ResMut, Transform};
use bevy_prototype_debug_lines::DebugLines;
use bevy_rapier3d::prelude::Collider;

use crate::{components::DebugRenderColor, resources::DebugRenderConfig};

pub fn debug_render_collider_system(
    mut commands: Commands,
    debug_render_config: Res<DebugRenderConfig>,
    query_colliders: Query<(Entity, &Collider, &Transform, Option<&DebugRenderColor>)>,
    mut lines: ResMut<DebugLines>,
) {
    if !debug_render_config.colliders {
        return;
    }

    for (entity, collider, transform, render_color) in query_colliders.iter() {
        let color = if let Some(render_color) = render_color {
            render_color.color
        } else {
            let debug_render_color = DebugRenderColor::random();
            let color = debug_render_color.color;
            commands.entity(entity).insert(debug_render_color);
            color
        };

        if let Some(cuboid) = collider.as_cuboid() {
            let (vertices, indices) = cuboid.raw.to_outline();

            for idx in indices {
                lines.line_colored(
                    transform.mul_vec3(vertices[idx[0] as usize].into()),
                    transform.mul_vec3(vertices[idx[1] as usize].into()),
                    0.0,
                    color,
                );
            }
        }
    }
}
