use bevy::{
    input::Input,
    prelude::{
        App, Camera, Camera3d, GlobalTransform, Handle, MouseButton, Plugin, Query, Res, ResMut,
        With,
    },
    render::camera::Projection,
    window::Windows,
};
use bevy_egui::EguiContext;
use bevy_inspector_egui::{InspectableRegistry, WorldInspectorParams};
use bevy_rapier3d::prelude::{InteractionGroups, QueryFilter, RapierContext};

use crate::{
    components::{
        ColliderEntity, ColliderParent, ZoneObject, ZoneObjectAnimatedObject, ZoneObjectId,
        ZoneObjectPart, ZoneObjectPartCollisionShape, ZoneObjectTerrain,
        COLLISION_FILTER_INSPECTABLE,
    },
    ray_from_screenspace::ray_from_screenspace,
    render::{ObjectMaterial, ObjectMaterialBlend, ObjectMaterialGlow},
    resources::DebugInspector,
};

pub struct DebugInspectorPlugin;

impl Plugin for DebugInspectorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DebugInspector::default())
            .add_system(debug_inspector_picking_system);

        let mut inspectable_registry = app
            .world
            .get_resource_or_insert_with(InspectableRegistry::default);
        inspectable_registry.register::<ColliderEntity>();
        inspectable_registry.register::<ColliderParent>();
        inspectable_registry.register::<ZoneObject>();
        inspectable_registry.register::<ZoneObjectTerrain>();
        inspectable_registry.register::<ZoneObjectAnimatedObject>();
        inspectable_registry.register::<ZoneObjectPart>();
        inspectable_registry.register::<ZoneObjectPartCollisionShape>();
        inspectable_registry.register::<ZoneObjectId>();
        inspectable_registry.register::<ObjectMaterial>();
        inspectable_registry.register::<ObjectMaterialBlend>();
        inspectable_registry.register::<ObjectMaterialGlow>();
        inspectable_registry.register::<Handle<ObjectMaterial>>();

        let mut world_inspector_params = app
            .world
            .get_resource_or_insert_with(WorldInspectorParams::default);
        world_inspector_params.ignore_component::<bevy::render::primitives::Aabb>();
    }
}

#[allow(clippy::too_many_arguments)]
fn debug_inspector_picking_system(
    mut debug_inspector_state: ResMut<DebugInspector>,
    mut egui_ctx: ResMut<EguiContext>,
    mouse_button_input: Res<Input<MouseButton>>,
    rapier_context: Res<RapierContext>,
    windows: Res<Windows>,
    query_camera: Query<(&Camera, &Projection, &GlobalTransform), With<Camera3d>>,
) {
    if !debug_inspector_state.enable_picking {
        // Picking disabled
        return;
    }

    let cursor_position = windows.primary().cursor_position();
    if cursor_position.is_none() || egui_ctx.ctx_mut().wants_pointer_input() {
        // Mouse not in window, or is over UI
        return;
    }
    let cursor_position = cursor_position.unwrap();

    if mouse_button_input.just_pressed(MouseButton::Middle) {
        for (camera, camera_projection, camera_transform) in query_camera.iter() {
            if let Some((ray_origin, ray_direction)) = ray_from_screenspace(
                cursor_position,
                &windows,
                camera,
                camera_projection,
                camera_transform,
            ) {
                if let Some((collider_entity, _distance)) = rapier_context.cast_ray(
                    ray_origin,
                    ray_direction,
                    10000000.0,
                    false,
                    QueryFilter::new().groups(
                        InteractionGroups::all().with_memberships(COLLISION_FILTER_INSPECTABLE),
                    ),
                ) {
                    debug_inspector_state.entity = Some(collider_entity);
                }
            }
        }
    }
}
