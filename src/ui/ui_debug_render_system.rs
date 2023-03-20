use bevy::{
    hierarchy::Children,
    prelude::{Assets, Handle, Local, Query, ResMut, With},
};
use bevy_egui::{egui, EguiContexts};

use crate::{
    components::{EventObject, WarpObject},
    render::ObjectMaterial,
    resources::DebugRenderConfig,
    ui::UiStateDebugWindows,
};

#[derive(Default)]
pub struct UiStateDebugRender {
    pub render_event_objects: bool,
    pub render_warp_objects: bool,
}

pub fn ui_debug_render_system(
    mut egui_context: EguiContexts,
    mut ui_state_debug_windows: ResMut<UiStateDebugWindows>,
    mut ui_state_debug_render: Local<UiStateDebugRender>,
    mut debug_render_config: ResMut<DebugRenderConfig>,
    query_event_objects: Query<&Children, With<EventObject>>,
    query_warp_objects: Query<&Children, With<WarpObject>>,
    query_object_material: Query<&Handle<ObjectMaterial>>,
    mut object_materials: ResMut<Assets<ObjectMaterial>>,
) {
    if !ui_state_debug_windows.debug_ui_open {
        return;
    }

    egui::Window::new("Debug Render")
        .open(&mut ui_state_debug_windows.debug_render_open)
        .show(egui_context.ctx_mut(), |ui| {
            ui.checkbox(&mut debug_render_config.colliders, "Show Colliders");
            ui.checkbox(&mut debug_render_config.skeleton, "Show Skeletons");
            ui.checkbox(&mut debug_render_config.bone_up, "Show Bone Up");
            ui.checkbox(
                &mut debug_render_config.directional_light_frustum,
                "Show Directional Light Frustum",
            );

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
                            if let Ok(handle) = query_object_material.get(*child_entity) {
                                if let Some(mut material) = object_materials.get_mut(handle) {
                                    material.alpha_value = Some(0.75);
                                }
                            }
                        }
                    }
                } else {
                    for children in query_event_objects.iter() {
                        for child_entity in children.iter() {
                            if let Ok(handle) = query_object_material.get(*child_entity) {
                                if let Some(mut material) = object_materials.get_mut(handle) {
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
                            if let Ok(handle) = query_object_material.get(*child_entity) {
                                if let Some(mut material) = object_materials.get_mut(handle) {
                                    material.alpha_value = Some(0.75);
                                }
                            }
                        }
                    }
                } else {
                    for children in query_warp_objects.iter() {
                        for child_entity in children.iter() {
                            if let Ok(handle) = query_object_material.get(*child_entity) {
                                if let Some(mut material) = object_materials.get_mut(handle) {
                                    material.alpha_value = None;
                                }
                            }
                        }
                    }
                }
            }
        });
}
