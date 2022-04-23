use bevy::prelude::{Commands, Entity, Local, Or, Query, ResMut, With};
use bevy_egui::{egui, EguiContext};
use bevy_rapier3d::prelude::ColliderShapeComponent;

use crate::{
    components::{CharacterModel, DebugRenderCollider, DebugRenderSkeleton, NpcModel},
    ui::UiStateDebugWindows,
};

#[derive(Default)]
pub struct UiStateDebugRender {
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
) {
    if !ui_state_debug_windows.debug_ui_open {
        return;
    }

    egui::Window::new("Debug Render")
        .open(&mut ui_state_debug_windows.debug_render_open)
        .show(egui_context.ctx_mut(), |ui| {
            egui::Grid::new("debug_render_settings")
                .num_columns(2)
                .show(ui, |ui| {
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
                });
        });
}
