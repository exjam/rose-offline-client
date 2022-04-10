use bevy::prelude::{Entity, Mut, World};
use bevy_egui::{egui, EguiContext};
use bevy_inspector_egui::{Context, Inspectable};

use crate::{resources::DebugInspector, ui::UiStateDebugWindows};

pub fn ui_debug_entity_inspector_system(world: &mut World) {
    world.resource_scope(
        |world, mut ui_state_debug_windows: Mut<UiStateDebugWindows>| {
            world.resource_scope(|world, mut debug_inspector_state: Mut<DebugInspector>| {
                if !ui_state_debug_windows.object_inspector_open {
                    return;
                }

                let mut egui_context = world.get_resource_mut::<EguiContext>().unwrap();
                let ctx = egui_context.ctx_mut().clone();

                let mut context = Context::new_world_access(Some(&ctx), world);

                // This manually circumcents bevy's change detection and probably isn't sound.
                // Todo: add bevy API to allow this safely
                #[allow(clippy::cast_ref_to_mut)]
                let value = unsafe {
                    &mut *(debug_inspector_state.as_ref() as *const DebugInspector
                        as *mut DebugInspector)
                };

                let mut changed = false;
                egui::Window::new("Entity Inspector")
                    .open(&mut ui_state_debug_windows.object_inspector_open)
                    .resizable(true)
                    .vscroll(true)
                    .show(&ctx, |ui| {
                        ui.style_mut().wrap = Some(false);

                        ui.checkbox(
                            &mut value.enable_picking,
                            "Enable Picking (with middle mouse button)",
                        );
                        ui.separator();

                        changed = value.entity.ui(
                            ui,
                            <Option<Entity> as Inspectable>::Attributes::default(),
                            &mut context,
                        );
                    });

                if changed {
                    // trigger change detection
                    debug_inspector_state.as_mut();
                }
            });
        },
    );
}
