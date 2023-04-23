use bevy::{
    prelude::{Camera3d, DirectionalLight, Entity, Mut, With, World},
    window::PrimaryWindow,
};
use bevy_egui::EguiContext;

use crate::{components::PlayerCharacter, resources::DebugInspector, ui::UiStateDebugWindows};

pub fn ui_debug_entity_inspector_system(world: &mut World) {
    let mut egui_context = world
        .query_filtered::<&mut EguiContext, With<PrimaryWindow>>()
        .single(world)
        .clone();

    world.resource_scope(
        |world, mut ui_state_debug_windows: Mut<UiStateDebugWindows>| {
            world.resource_scope(|world, mut debug_inspector_state: Mut<DebugInspector>| {
                if !ui_state_debug_windows.object_inspector_open {
                    return;
                }

                egui::Window::new("Entity Inspector")
                    .open(&mut ui_state_debug_windows.object_inspector_open)
                    .resizable(true)
                    .vscroll(true)
                    .show(egui_context.get_mut(), |ui| {
                        ui.style_mut().wrap = Some(false);

                        ui.horizontal(|ui| {
                            if ui.button("Camera").clicked() {
                                debug_inspector_state.entity = Some(
                                    world
                                        .query_filtered::<Entity, With<Camera3d>>()
                                        .single(world),
                                );
                            }

                            if ui.button("Player").clicked() {
                                debug_inspector_state.entity = Some(
                                    world
                                        .query_filtered::<Entity, With<PlayerCharacter>>()
                                        .single(world),
                                );
                            }

                            if ui.button("Light").clicked() {
                                debug_inspector_state.entity = Some(
                                    world
                                        .query_filtered::<Entity, With<DirectionalLight>>()
                                        .single(world),
                                );
                            }
                        });

                        let mut enable_picking = debug_inspector_state.enable_picking;
                        ui.checkbox(&mut enable_picking, "Enable Picking (with P key)");
                        if enable_picking != debug_inspector_state.enable_picking {
                            debug_inspector_state.enable_picking = enable_picking;
                        }
                        ui.separator();

                        if let Some(entity) = debug_inspector_state.entity {
                            bevy_inspector_egui::bevy_inspector::ui_for_entity(world, entity, ui);
                        }
                    });
            });
        },
    );
}
