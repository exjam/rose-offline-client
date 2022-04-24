use bevy::{
    input::Input,
    prelude::{
        App, Camera, GlobalTransform, Handle, MouseButton, Plugin, Query, Res, ResMut, With,
    },
    render::camera::Camera3d,
    window::Windows,
};
use bevy_egui::EguiContext;
use bevy_inspector_egui::{InspectableRegistry, WorldInspectorParams};
use bevy_rapier3d::{
    physics::{
        IntoEntity, QueryPipelineColliderComponentsQuery, QueryPipelineColliderComponentsSet,
    },
    prelude::{InteractionGroups, QueryPipeline},
};

use crate::{
    components::COLLISION_FILTER_INSPECTABLE, render::StaticMeshMaterial, resources::DebugInspector,
};

use super::{
    collision_system::ray_from_screenspace,
    load_zone_system::{
        ZoneObjectAnimatedObject, ZoneObjectPart, ZoneObjectPartCollisionShape, ZoneObjectTerrain,
    },
    ZoneObject,
};

pub struct DebugInspectorPlugin;

impl Plugin for DebugInspectorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DebugInspector::default())
            .add_system(debug_inspector_picking_system);

        let mut inspectable_registry = app
            .world
            .get_resource_or_insert_with(InspectableRegistry::default);
        inspectable_registry.register::<StaticMeshMaterial>();
        inspectable_registry.register::<Handle<StaticMeshMaterial>>();
        inspectable_registry.register::<ZoneObject>();
        inspectable_registry.register::<ZoneObjectTerrain>();
        inspectable_registry.register::<ZoneObjectAnimatedObject>();
        inspectable_registry.register::<ZoneObjectPart>();
        inspectable_registry.register::<ZoneObjectPartCollisionShape>();

        let mut world_inspector_params = app
            .world
            .get_resource_or_insert_with(WorldInspectorParams::default);
        world_inspector_params.ignore_component::<bevy::render::primitives::Aabb>();
        world_inspector_params.ignore_component::<bevy_rapier3d::prelude::ColliderTypeComponent>();
        world_inspector_params.ignore_component::<bevy_rapier3d::prelude::ColliderShapeComponent>();
        world_inspector_params
            .ignore_component::<bevy_rapier3d::prelude::ColliderPositionComponent>();
        world_inspector_params
            .ignore_component::<bevy_rapier3d::prelude::ColliderMaterialComponent>();
        world_inspector_params.ignore_component::<bevy_rapier3d::prelude::ColliderFlagsComponent>();
        world_inspector_params
            .ignore_component::<bevy_rapier3d::prelude::ColliderMassPropsComponent>();
        world_inspector_params
            .ignore_component::<bevy_rapier3d::prelude::ColliderChangesComponent>();
        world_inspector_params
            .ignore_component::<bevy_rapier3d::prelude::ColliderBroadPhaseDataComponent>();
    }
}

#[allow(clippy::too_many_arguments)]
fn debug_inspector_picking_system(
    mut debug_inspector_state: ResMut<DebugInspector>,
    mut egui_ctx: ResMut<EguiContext>,
    query_camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mouse_button_input: Res<Input<MouseButton>>,
    windows: Res<Windows>,
    colliders: QueryPipelineColliderComponentsQuery,
    query_pipeline: Res<QueryPipeline>,
) {
    if !debug_inspector_state.enable_picking {
        // Picking disabled
        return;
    }

    let colliders = QueryPipelineColliderComponentsSet(&colliders);
    let cursor_position = windows.primary().cursor_position();
    if cursor_position.is_none() || egui_ctx.ctx_mut().wants_pointer_input() {
        // Mouse not in window, or is over UI
        return;
    }
    let cursor_position = cursor_position.unwrap();

    if mouse_button_input.just_pressed(MouseButton::Middle) {
        for (camera, camera_transform) in query_camera.iter() {
            if let Some(ray) =
                ray_from_screenspace(cursor_position, &windows, camera, camera_transform)
            {
                let hit = query_pipeline.cast_ray(
                    &colliders,
                    &ray,
                    10000000.0,
                    false,
                    InteractionGroups::all().with_memberships(COLLISION_FILTER_INSPECTABLE),
                    None,
                );

                if let Some((hit_object, _distance)) = hit {
                    debug_inspector_state.entity = Some(hit_object.0.entity());
                }
            }
        }
    }
}
