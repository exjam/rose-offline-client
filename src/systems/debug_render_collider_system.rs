use bevy::prelude::{Entity, Gizmos, GlobalTransform, Query, Res};
use bevy_rapier3d::prelude::Collider;

use crate::resources::DebugRenderConfig;

pub fn debug_render_collider_system(
    debug_render_config: Res<DebugRenderConfig>,
    query_colliders: Query<(Entity, &Collider, &GlobalTransform)>,
    mut gizmos: Gizmos,
) {
    if !debug_render_config.colliders {
        return;
    }

    for (entity, collider, global_transform) in query_colliders.iter() {
        if let Some(cuboid) = collider.as_cuboid() {
            let transform = global_transform
                .compute_transform()
                .with_scale(cuboid.half_extents() * 2.0);
            let color = debug_render_config.color_for_entity(entity);
            gizmos.cuboid(transform, color);
        }
    }
}
