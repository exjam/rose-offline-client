use bevy::{
    input::{
        mouse::{MouseMotion, MouseWheel},
        Input,
    },
    prelude::{App, EventReader, EventWriter, KeyCode, MouseButton, Plugin, Query, Res, ResMut},
};
use bevy_egui::EguiContext;
use smooth_bevy_cameras::controllers::unreal::{
    default_input_map, ControlEvent, UnrealCameraBundle, UnrealCameraController, UnrealCameraPlugin,
};

#[derive(Default)]
pub struct FlyCameraPlugin;

pub type FlyCameraBundle = UnrealCameraBundle;
pub type FlyCameraController = UnrealCameraController;

impl Plugin for FlyCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(UnrealCameraPlugin {
            override_input_system: true,
        })
        .add_system(egui_blocking_default_input_map);
    }
}

fn egui_blocking_default_input_map(
    events: EventWriter<ControlEvent>,
    mouse_wheel_reader: EventReader<MouseWheel>,
    mouse_motion_events: EventReader<MouseMotion>,
    keyboard: Res<Input<KeyCode>>,
    mouse_buttons: Res<Input<MouseButton>>,
    controllers: Query<&mut FlyCameraController>,
    mut egui_ctx: ResMut<EguiContext>,
) {
    if egui_ctx.ctx_mut().wants_pointer_input() || egui_ctx.ctx_mut().wants_keyboard_input() {
        return;
    }

    default_input_map(
        events,
        mouse_wheel_reader,
        mouse_motion_events,
        keyboard,
        mouse_buttons,
        controllers,
    );
}
