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
use bevy_rapier3d::prelude::{CollisionGroups, Group, QueryFilter, RapierContext};

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

        app.register_type::<ColliderEntity>()
            .register_type::<ColliderParent>()
            .register_type::<ZoneObject>()
            .register_type::<ZoneObjectTerrain>()
            .register_type::<ZoneObjectAnimatedObject>()
            .register_type::<ZoneObjectPart>()
            .register_type::<ZoneObjectPartCollisionShape>()
            .register_type::<ZoneObjectId>()
            .register_type::<ObjectMaterial>()
            .register_type::<ObjectMaterialBlend>()
            .register_type::<ObjectMaterialGlow>()
            .register_type::<Handle<ObjectMaterial>>();

        app.add_plugin(bevy_inspector_egui::DefaultInspectorConfigPlugin);
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
                    QueryFilter::new().groups(CollisionGroups::new(
                        COLLISION_FILTER_INSPECTABLE,
                        Group::all(),
                    )),
                ) {
                    debug_inspector_state.entity = Some(collider_entity);
                }
            }
        }
    }
}
