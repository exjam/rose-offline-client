use bevy::prelude::World;

pub fn ui_debug_entity_inspector_system(world: &mut World) {
    /*
    TODO: Fix ui_debug_entity_inspector_system

    world.resource_scope(
        |world, mut ui_state_debug_windows: Mut<UiStateDebugWindows>| {
            world.resource_scope(|world, mut debug_inspector_state: Mut<DebugInspector>| {
                if !ui_state_debug_windows.object_inspector_open {
                    return;
                }

                let mut egui_context = world.get_resource_mut::<EguiContexts>().unwrap();
                let ctx = egui_context.ctx_mut().clone();

                egui::Window::new("Entity Inspector")
                    .open(&mut ui_state_debug_windows.object_inspector_open)
                    .resizable(true)
                    .vscroll(true)
                    .show(&ctx, |ui| {
                        ui.style_mut().wrap = Some(false);

                        let mut enable_picking = debug_inspector_state.enable_picking;
                        ui.checkbox(
                            &mut enable_picking,
                            "Enable Picking (with middle mouse button)",
                        );
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
    */
}
