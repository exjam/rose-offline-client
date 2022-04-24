use bevy::{
    hierarchy::Children,
    prelude::{Assets, Commands, Entity, Handle, Local, Or, Query, ResMut, With},
};
use bevy_egui::{egui, EguiContext};
use bevy_rapier3d::prelude::ColliderShapeComponent;

use crate::{
    components::{
        CharacterModel, DebugRenderCollider, DebugRenderSkeleton, EventObject, NpcModel, WarpObject,
    },
    render::StaticMeshMaterial,
    ui::UiStateDebugWindows,
};

#[derive(Default)]
pub struct UiStateDebugRender {
    pub render_event_objects: bool,
    pub render_warp_objects: bool,
    pub render_skeletons: bool,
    pub render_colliders: bool,
}

pub fn ui_debug_render_system(
    mut commands: Commands,
    mut egui_context: ResMut<EguiContext>,
    mut ui_state_debug_windows: ResMut<UiStateDebugWindows>,
    mut ui_state_debug_render: Local<UiStateDebugRender>,
    query_add_colliders: Query<Entity, With<ColliderShapeComponent>>,
    query_add_skeletons: Query<Entity, Or<(With<CharacterModel>, With<NpcModel>)>>,
    query_remove_colliders: Query<Entity, With<DebugRenderCollider>>,
    query_remove_skeletons: Query<Entity, With<DebugRenderSkeleton>>,
    query_event_objects: Query<&Children, With<EventObject>>,
    query_warp_objects: Query<&Children, With<WarpObject>>,
    query_static_mesh_material: Query<&Handle<StaticMeshMaterial>>,
    mut static_mesh_materials: ResMut<Assets<StaticMeshMaterial>>,
) {
    if !ui_state_debug_windows.debug_ui_open {
        return;
    }

    egui::Window::new("Debug Render")
        .open(&mut ui_state_debug_windows.debug_render_open)
        .show(egui_context.ctx_mut(), |ui| {
            if ui
                .checkbox(
                    &mut ui_state_debug_render.render_colliders,
                    "Show Colliders",
                )
                .clicked()
            {
                if ui_state_debug_render.render_colliders {
                    for entity in query_add_colliders.iter() {
                        commands
                            .entity(entity)
                            .insert(DebugRenderCollider::default());
                    }
                } else {
                    for entity in query_remove_colliders.iter() {
                        commands.entity(entity).remove::<DebugRenderCollider>();
                    }
                }
            }

            if ui
                .checkbox(
                    &mut ui_state_debug_render.render_skeletons,
                    "Show Skeletons",
                )
                .clicked()
            {
                if ui_state_debug_render.render_skeletons {
                    for entity in query_add_skeletons.iter() {
                        commands
                            .entity(entity)
                            .insert(DebugRenderSkeleton::default());
                    }
                } else {
                    for entity in query_remove_skeletons.iter() {
                        commands.entity(entity).remove::<DebugRenderSkeleton>();
                    }
                }
            }

            if ui
                .checkbox(
                    &mut ui_state_debug_render.render_event_objects,
                    "Show Event Objects",
                )
                .clicked()
            {
                if ui_state_debug_render.render_event_objects {
                    for children in query_event_objects.iter() {
                        for child_entity in children.iter() {
                            if let Ok(handle) = query_static_mesh_material.get(*child_entity) {
                                if let Some(mut material) = static_mesh_materials.get_mut(handle) {
                                    material.alpha_value = Some(0.75);
                                }
                            }
                        }
                    }
                } else {
                    for children in query_event_objects.iter() {
                        for child_entity in children.iter() {
                            if let Ok(handle) = query_static_mesh_material.get(*child_entity) {
                                if let Some(mut material) = static_mesh_materials.get_mut(handle) {
                                    material.alpha_value = None;
                                }
                            }
                        }
                    }
                }
            }

            if ui
                .checkbox(
                    &mut ui_state_debug_render.render_warp_objects,
                    "Show Warp Objects",
                )
                .clicked()
            {
                if ui_state_debug_render.render_warp_objects {
                    for children in query_warp_objects.iter() {
                        for child_entity in children.iter() {
                            if let Ok(handle) = query_static_mesh_material.get(*child_entity) {
                                if let Some(mut material) = static_mesh_materials.get_mut(handle) {
                                    material.alpha_value = Some(0.75);
                                }
                            }
                        }
                    }
                } else {
                    for children in query_warp_objects.iter() {
                        for child_entity in children.iter() {
                            if let Ok(handle) = query_static_mesh_material.get(*child_entity) {
                                if let Some(mut material) = static_mesh_materials.get_mut(handle) {
                                    material.alpha_value = None;
                                }
                            }
                        }
                    }
                }
            }
        });
}
