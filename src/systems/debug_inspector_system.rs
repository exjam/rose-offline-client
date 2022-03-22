use bevy::{
    ecs::event::Events,
    input::Input,
    prelude::{
        App, Camera, Entity, EventReader, GlobalTransform, Handle, MouseButton, Plugin, Query, Res,
        ResMut, With,
    },
    render::camera::Camera3d,
    window::Windows,
};
use bevy_egui::EguiContext;
use bevy_inspector_egui::{
    plugin::InspectorWindows, Inspectable, InspectableRegistry, InspectorPlugin,
    WorldInspectorParams,
};
use bevy_rapier3d::{
    physics::{
        IntoEntity, QueryPipelineColliderComponentsQuery, QueryPipelineColliderComponentsSet,
    },
    prelude::{InteractionGroups, QueryPipeline},
};

use crate::{
    components::COLLISION_FILTER_INSPECTABLE, events::DebugInspectorEvent,
    render::StaticMeshMaterial,
};

use super::{
    collision_system::ray_from_screenspace,
    load_zone_system::{
        ZoneObjectAnimatedObject, ZoneObjectStaticObjectPart,
        ZoneObjectStaticObjectPartCollisionShape, ZoneObjectTerrain,
    },
    ZoneObject,
};

#[derive(Inspectable, Default)]
pub struct DebugInspectorState {
    #[inspectable(label = "Picking")]
    pub enable_picking: bool,

    #[inspectable(label = "Entity")]
    pub entity: Option<Entity>,
}

pub struct DebugInspectorPlugin;

impl Plugin for DebugInspectorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(InspectorPlugin::<DebugInspectorState>::new())
            .insert_resource(Events::<DebugInspectorEvent>::default())
            .add_system(debug_inspector_picking_system)
            .add_system(debug_inspector_control_system);

        let mut inspector_windows = app.world.resource_mut::<InspectorWindows>();
        let mut window_data = inspector_windows.window_data_mut::<DebugInspectorState>();
        window_data.visible = false;
        window_data.name = "Entity Inspector".to_string();

        let mut inspectable_registry = app.world.resource_mut::<InspectableRegistry>();
        inspectable_registry.register::<StaticMeshMaterial>();
        inspectable_registry.register::<Handle<StaticMeshMaterial>>();
        inspectable_registry.register::<ZoneObject>();
        inspectable_registry.register::<ZoneObjectTerrain>();
        inspectable_registry.register::<ZoneObjectAnimatedObject>();
        inspectable_registry.register::<ZoneObjectStaticObjectPart>();
        inspectable_registry.register::<ZoneObjectStaticObjectPartCollisionShape>();

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

fn debug_inspector_control_system(
    mut events: EventReader<DebugInspectorEvent>,
    mut inspect_entity: ResMut<DebugInspectorState>,
    mut inspector_windows: ResMut<InspectorWindows>,
) {
    for event in events.iter() {
        match event {
            DebugInspectorEvent::Show => {
                inspector_windows
                    .window_data_mut::<DebugInspectorState>()
                    .visible = true;
            }
            DebugInspectorEvent::Hide => {
                inspector_windows
                    .window_data_mut::<DebugInspectorState>()
                    .visible = false;
            }
            &DebugInspectorEvent::InspectEntity(entity) => {
                inspect_entity.entity = Some(entity);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn debug_inspector_picking_system(
    mouse_button_input: Res<Input<MouseButton>>,
    windows: Res<Windows>,
    query_camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    query_pipeline: Res<QueryPipeline>,
    colliders: QueryPipelineColliderComponentsQuery,
    mut inspect_entity: ResMut<DebugInspectorState>,
    inspector_windows: Res<InspectorWindows>,
    mut egui_ctx: ResMut<EguiContext>,
) {
    if !inspector_windows
        .window_data::<DebugInspectorState>()
        .visible
        || !inspect_entity.enable_picking
    {
        // Inspector not open
        return;
    }

    let colliders = QueryPipelineColliderComponentsSet(&colliders);
    let cursor_position = windows.primary().cursor_position();
    if cursor_position.is_none() || egui_ctx.ctx_mut().wants_pointer_input() {
        // Mouse not in window, or is over UI
        return;
    }
    let cursor_position = cursor_position.unwrap();

    if mouse_button_input.just_pressed(MouseButton::Left) {
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
                    inspect_entity.entity = Some(hit_object.0.entity());
                }
            }
        }
    }
}
