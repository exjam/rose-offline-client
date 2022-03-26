use bevy::{
    math::Vec3,
    prelude::{
        Camera, Commands, Entity, EventWriter, Local, PerspectiveCameraBundle, Query, Res, ResMut,
        Transform, With,
    },
    render::camera::Camera3d,
};
use bevy_egui::{egui, EguiContext};

use crate::{
    components::ActiveMotion,
    events::{DebugInspectorEvent, LoadZoneEvent},
    fly_camera::{FlyCameraBundle, FlyCameraController},
    follow_camera::FollowCameraController,
    resources::GameData,
};

pub struct ZoneViewerUiState {
    zone_list_open: bool,
}

impl Default for ZoneViewerUiState {
    fn default() -> Self {
        Self {
            zone_list_open: true,
        }
    }
}

pub fn zone_viewer_setup_system(
    mut commands: Commands,
    query_cameras: Query<Entity, With<Camera3d>>,
    mut debug_inspector_events: EventWriter<DebugInspectorEvent>,
) {
    // Reset camera
    for entity in query_cameras.iter() {
        commands
            .entity(entity)
            .remove::<FollowCameraController>()
            .remove::<ActiveMotion>()
            .insert_bundle(FlyCameraBundle::new(
                FlyCameraController::default(),
                PerspectiveCameraBundle::default(),
                Vec3::new(5120.0, 50.0, -5120.0),
                Vec3::new(5200.0, 0.0, -5200.0),
            ));
    }

    debug_inspector_events.send(DebugInspectorEvent::Show);
}

#[allow(clippy::too_many_arguments)]
pub fn zone_viewer_system(
    mut ui_state: Local<ZoneViewerUiState>,
    game_data: Res<GameData>,
    mut load_zone_events: EventWriter<LoadZoneEvent>,
    mut egui_context: ResMut<EguiContext>,
    camera_query: Query<&Transform, With<Camera>>,
) {
    egui::Window::new("Camera").show(egui_context.ctx_mut(), |ui| {
        let transform = camera_query.single();
        ui.label(format!("Translation: {}", transform.translation));
        ui.label(format!("Forward: {}", transform.forward()));
    });

    egui::Window::new("Zone List")
        .vscroll(true)
        .resizable(true)
        .default_height(300.0)
        .open(&mut ui_state.zone_list_open)
        .show(egui_context.ctx_mut(), |ui| {
            egui::Grid::new("zone_list_grid")
                .num_columns(3)
                .show(ui, |ui| {
                    ui.label("id");
                    ui.label("name");
                    ui.end_row();

                    for zone in game_data.zone_list.iter() {
                        ui.label(format!("{}", zone.id.get()));
                        ui.label(&zone.name);
                        if ui.button("Load").clicked() {
                            load_zone_events.send(LoadZoneEvent::new(zone.id));
                        }
                        ui.end_row();
                    }
                });
        });
}
