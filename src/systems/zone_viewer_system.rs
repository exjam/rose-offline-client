use bevy::{
    ecs::query::QueryEntityError,
    math::Vec3,
    prelude::{
        Assets, Camera, Color, Commands, Entity, EventReader, GlobalTransform, Handle, Local, Mesh,
        PerspectiveCameraBundle, Query, Res, ResMut, Transform, With,
    },
    render::mesh::{Indices, VertexAttributeValues},
    window::Windows,
};
use bevy_egui::{egui, EguiContext};
use bevy_mod_picking::{PickingCameraBundle, PickingEvent, PickingPluginsState};
use bevy_polyline::{Polyline, PolylineBundle, PolylineMaterial};
use rose_data::ZoneList;

use crate::{bevy_flycam::FlyCam, resources::LoadedZone};

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

#[derive(Default)]
pub struct ObjectInspectorState {
    inspecting_entity: Option<Entity>,
    outline_entity: Option<Entity>,
}

pub fn zone_viewer_setup_system(
    mut commands: Commands,
    query_cameras: Query<Entity, With<Camera>>,
) {
    // Remove any other cameras
    for entity in query_cameras.iter() {
        commands.entity(entity).despawn();
    }

    commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_xyz(5120.0, 50.0, -5120.0)
                .looking_at(Vec3::new(5200.0, 0.0, -5200.0), Vec3::Y),
            ..Default::default()
        })
        .insert_bundle(PickingCameraBundle::default())
        .insert(FlyCam);
}

#[allow(clippy::too_many_arguments)]
pub fn zone_viewer_system(
    mut commands: Commands,
    mut ui_state: Local<ZoneViewerUiState>,
    mut object_inspector_state: Local<ObjectInspectorState>,
    query_picking: Query<(&Handle<Mesh>, &GlobalTransform)>,
    query_zone_object: Query<&ZoneObject>,
    mut picking_events: EventReader<PickingEvent>,
    windows: Res<Windows>,
    zone_list: Res<ZoneList>,
    mut loaded_zone: ResMut<LoadedZone>,
    mut picking: ResMut<PickingPluginsState>,
    mut egui_context: ResMut<EguiContext>,
    meshes: Res<Assets<Mesh>>,
    mut polylines: ResMut<Assets<Polyline>>,
    mut polyline_materials: ResMut<Assets<PolylineMaterial>>,
    camera_query: Query<&Transform, With<Camera>>,
) {
    // Disable picking when controlling camera
    let window = windows.get_primary().unwrap();
    let cursor_locked = window.cursor_locked();
    picking.enable_picking = !cursor_locked;
    picking.update_debug_cursor = !cursor_locked;

    // Handle mouse picking events for object inspector
    for event in picking_events.iter() {
        if let &PickingEvent::Clicked(entity) = event {
            if let Ok((mesh, &global_transform)) = query_picking.get(entity) {
                if let Some(mesh) = meshes.get(mesh) {
                    object_inspector_state.inspecting_entity = Some(entity);

                    if let (
                        Some(Indices::U16(indices)),
                        Some(VertexAttributeValues::Float32x3(vertices)),
                    ) = (mesh.indices(), mesh.attribute(Mesh::ATTRIBUTE_POSITION))
                    {
                        let mut polyline_vertices = Vec::new();
                        for &i in indices.iter() {
                            let vertex = vertices[i as usize];
                            polyline_vertices.push(Vec3::new(vertex[0], vertex[1], vertex[2]));
                        }

                        if let Some(outline_entity) = object_inspector_state.outline_entity.take() {
                            commands.entity(outline_entity).despawn();
                        }

                        object_inspector_state.outline_entity = Some(
                            commands
                                .spawn_bundle(PolylineBundle {
                                    polyline: polylines.add(Polyline {
                                        vertices: polyline_vertices,
                                    }),
                                    material: polyline_materials.add(PolylineMaterial {
                                        width: 4.0,
                                        color: Color::PINK,
                                        perspective: true,
                                    }),
                                    transform: global_transform.into(),
                                    ..Default::default()
                                })
                                .id(),
                        );
                    }
                }
            }
        }
    }

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

    if let Some(inspecting_entity) = object_inspector_state.inspecting_entity {
        match query_zone_object.get(inspecting_entity) {
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
                object_inspector_state.inspecting_entity = None;

                if let Some(outline_entity) = object_inspector_state.outline_entity.take() {
                    commands.entity(outline_entity).despawn();
                }
            }
            Err(_) => {}
        }
    }
}
