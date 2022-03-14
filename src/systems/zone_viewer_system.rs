use bevy::{
    ecs::query::QueryEntityError,
    math::Vec3,
    prelude::{
        Assets, Camera, Color, Commands, Entity, EventReader, Local, PerspectiveCameraBundle,
        PerspectiveProjection, Query, Res, ResMut, Transform, With,
    },
};
use bevy_egui::{egui, EguiContext};
use bevy_polyline::{Polyline, PolylineBundle, PolylineMaterial};
use bevy_rapier3d::prelude::{ColliderShapeComponent, AABB};
use rose_data::ZoneList;
use smooth_bevy_cameras::controllers::unreal::{UnrealCameraBundle, UnrealCameraController};

use crate::{events::PickingEvent, resources::LoadedZone};

use super::load_zone_system::ZoneObject;

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

pub struct ZoneViewerInspectObject {
    entity: Entity,
    outline_entity: Entity,
}

pub fn zone_viewer_setup_system(
    mut commands: Commands,
    query_cameras: Query<Entity, (With<Camera>, With<PerspectiveProjection>)>,
) {
    // Remove any other cameras
    for entity in query_cameras.iter() {
        commands.entity(entity).despawn();
    }

    commands.spawn_bundle(UnrealCameraBundle::new(
        UnrealCameraController::default(),
        PerspectiveCameraBundle::default(),
        Vec3::new(5120.0, 50.0, -5120.0),
        Vec3::new(5200.0, 0.0, -5200.0),
    ));
}

pub fn zone_viewer_picking_system(
    mut commands: Commands,
    mut picking_events: EventReader<PickingEvent>,
    query: Query<&ColliderShapeComponent>,
    mut polylines: ResMut<Assets<Polyline>>,
    mut polyline_materials: ResMut<Assets<PolylineMaterial>>,
    inspect_object: Option<Res<ZoneViewerInspectObject>>,
) {
    if let Some(event) = picking_events.iter().last() {
        if let Ok(collider_shape) = query.get(event.entity) {
            if let Some(trimesh) = collider_shape.as_trimesh() {
                let mut polyline_vertices = Vec::new();
                let aabb_verts = trimesh.local_aabb().vertices();
                for (a, b) in AABB::EDGES_VERTEX_IDS {
                    polyline_vertices.push(aabb_verts[a].into());
                    polyline_vertices.push(aabb_verts[b].into());
                }

                // If we already were inspecting an object, despawn its outline we added
                if let Some(inspect_object) = inspect_object {
                    commands.entity(inspect_object.outline_entity).despawn();
                }

                let outline_entity = commands
                    .spawn_bundle(PolylineBundle {
                        polyline: polylines.add(Polyline {
                            vertices: polyline_vertices,
                        }),
                        material: polyline_materials.add(PolylineMaterial {
                            width: 4.0,
                            color: Color::PINK,
                            perspective: false,
                        }),
                        ..Default::default()
                    })
                    .id();

                commands.insert_resource(ZoneViewerInspectObject {
                    entity: event.entity,
                    outline_entity,
                });
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn zone_viewer_system(
    mut commands: Commands,
    mut ui_state: Local<ZoneViewerUiState>,
    inspect_object: Option<Res<ZoneViewerInspectObject>>,
    query_zone_object: Query<&ZoneObject>,
    zone_list: Res<ZoneList>,
    mut loaded_zone: ResMut<LoadedZone>,
    mut egui_context: ResMut<EguiContext>,
    camera_query: Query<&Transform, With<Camera>>,
) {
    // Draw ui
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
            egui::Grid::new("zone_list_grid").show(ui, |ui| {
                ui.label("id");
                ui.label("name");
                ui.end_row();

                for zone in zone_list.iter() {
                    ui.label(format!("{}", zone.id.get()));
                    ui.label(&zone.name);
                    if ui.button("Load").clicked() {
                        loaded_zone.next_zone_id = Some(zone.id);
                    }
                    ui.end_row();
                }
            });
        });

    if let Some(inspect_object) = inspect_object {
        match query_zone_object.get(inspect_object.entity) {
            Ok(zone_object) => {
                egui::Window::new("Object Inspector")
                    .vscroll(true)
                    .resizable(true)
                    .default_height(300.0)
                    .show(egui_context.ctx_mut(), |ui| {
                        egui::Grid::new("zone_list_grid").show(ui, |ui| {
                            ui.label("mesh");
                            ui.label(&zone_object.mesh_path);
                            ui.end_row();

                            ui.label("texture");
                            ui.label(zone_object.material.path.path().to_str().unwrap_or(""));
                            ui.end_row();

                            ui.label("alpha_enabled");
                            ui.label(format!("{}", zone_object.material.alpha_enabled));
                            ui.end_row();

                            ui.label("alpha_test");
                            ui.label(format!("{:?}", zone_object.material.alpha_test));
                            ui.end_row();

                            ui.label("alpha");
                            ui.label(format!("{:?}", zone_object.material.alpha));
                            ui.end_row();

                            ui.label("blend_mode");
                            ui.label(format!("{:?}", zone_object.material.blend_mode));
                            ui.end_row();

                            ui.label("glow");
                            ui.label(format!("{:?}", zone_object.material.glow));
                            ui.end_row();

                            ui.label("is_skin");
                            ui.label(format!("{}", zone_object.material.is_skin));
                            ui.end_row();

                            ui.label("specular_enabled");
                            ui.label(format!("{:?}", zone_object.material.specular_enabled));
                            ui.end_row();

                            ui.label("two_sided");
                            ui.label(format!("{}", zone_object.material.two_sided));
                            ui.end_row();

                            ui.label("z_write_enabled");
                            ui.label(format!("{}", zone_object.material.z_write_enabled));
                            ui.end_row();

                            ui.label("z_test_enabled");
                            ui.label(format!("{}", zone_object.material.z_test_enabled));
                            ui.end_row();
                        });
                    });
            }
            Err(QueryEntityError::NoSuchEntity) => {
                // Entity no longer valid, deselect
                commands.entity(inspect_object.outline_entity).despawn();
                commands.remove_resource::<ZoneViewerInspectObject>();
            }
            Err(_) => {}
        }
    }
}
