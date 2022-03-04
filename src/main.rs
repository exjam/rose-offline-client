use std::{path::Path, sync::Arc, time::Duration};

use bevy::{
    asset::AssetServerSettings,
    math::Vec3,
    prelude::{
        AddAsset, App, AssetServer, Assets, BuildChildren, Color, Commands, Component, Entity,
        EventReader, GlobalTransform, Handle, Mesh, Msaa, PerspectiveCameraBundle, Query, Res,
        ResMut, Transform, With,
    },
    render::mesh::{Indices, VertexAttributeValues},
    window::{WindowDescriptor, Windows},
};
mod bevy_flycam;
use bevy_flycam::{FlyCam, MovementSettings, NoCameraPlayerPlugin};
use bevy_mod_picking::{
    DebugCursorPickingPlugin, InteractablePickingPlugin, PickingCameraBundle, PickingEvent,
    PickingPlugin, PickingPluginsState,
};
use bevy_polyline::{Polyline, PolylineBundle, PolylineMaterial, PolylinePlugin};
use rose_file_readers::{VfsIndex, ZscMaterial};

mod render;
mod vfs_asset_io;
mod zms_asset_loader;
mod zone_loader;

use render::{
    RoseRenderPlugin, StaticMeshMaterial, TerrainMaterial, TextureArray, WaterMeshMaterial,
};
use vfs_asset_io::VfsAssetIo;
use zms_asset_loader::ZmsAssetLoader;

struct ClientConfiguration {
    zone_id: usize,
}

pub struct VfsResource {
    vfs: Arc<VfsIndex>,
}

fn main() {
    let mut command = clap::Command::new("bevy_rose")
        .arg(
            clap::Arg::new("data-idx")
                .long("data-idx")
                .help("Path to data.idx")
                .takes_value(true),
        )
        .arg(
            clap::Arg::new("data-path")
                .long("data-path")
                .help("Optional path to extracted data, any files here override ones in data.idx")
                .takes_value(true),
        )
        .arg(
            clap::Arg::new("zone")
                .long("zone")
                .help("Which zone to load")
                .takes_value(true),
        )
        .arg(
            clap::Arg::new("disable-vsync")
                .long("disable-vsync")
                .help("Disable v-sync to see accurate frame times"),
        );
    let data_path_error = command.error(
        clap::ErrorKind::ArgumentNotFound,
        "Must specify at least one of --data-idx or --data-path",
    );
    let matches = command.get_matches();

    let zone_id = matches
        .value_of("zone")
        .and_then(|str| str.parse::<usize>().ok())
        .unwrap_or(2);
    let disable_vsync = matches.is_present("disable-vsync");

    let mut data_idx_path = matches.value_of("data-idx").map(Path::new);
    let data_extracted_path = matches.value_of("data-path").map(Path::new);

    if data_idx_path.is_none() && data_extracted_path.is_none() {
        if Path::new("data.idx").exists() {
            data_idx_path = Some(Path::new("data.idx"));
        } else {
            data_path_error.exit();
        }
    }

    let vfs = Arc::new(
        VfsIndex::with_paths(data_idx_path, data_extracted_path).expect("Failed to initialise VFS"),
    );

    let mut app = App::new();

    // Initialise bevy engine
    app.insert_resource(Msaa { samples: 4 })
        .insert_resource(AssetServerSettings {
            asset_folder: data_extracted_path
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "data".to_string()),
            watch_for_changes: false,
        })
        .insert_resource(MovementSettings {
            sensitivity: 0.00012,
            speed: 200.,
        })
        .insert_resource(WindowDescriptor {
            title: "Definitely not a ROSE client".to_string(),
            present_mode: if disable_vsync {
                bevy::window::PresentMode::Immediate
            } else {
                bevy::window::PresentMode::Fifo
            },
            ..Default::default()
        })
        .add_plugin(bevy::log::LogPlugin::default())
        .add_plugin(bevy::core::CorePlugin::default())
        .add_plugin(bevy::diagnostic::LogDiagnosticsPlugin {
            wait_duration: Duration::from_secs(30),
            ..Default::default()
        })
        .add_plugin(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default())
        .add_plugin(bevy::transform::TransformPlugin::default())
        .add_plugin(bevy::diagnostic::DiagnosticsPlugin::default())
        .add_plugin(bevy::input::InputPlugin::default())
        .add_plugin(bevy::window::WindowPlugin::default());

    let task_pool = app.world.resource::<bevy::tasks::IoTaskPool>().0.clone();
    app.insert_resource(VfsResource { vfs: vfs.clone() })
        .insert_resource(AssetServer::new(VfsAssetIo::new(vfs), task_pool))
        .add_plugin(bevy::asset::AssetPlugin::default());

    app.add_plugin(bevy::scene::ScenePlugin::default())
        .add_plugin(bevy::winit::WinitPlugin::default())
        .add_plugin(bevy::render::RenderPlugin::default())
        .add_plugin(bevy::core_pipeline::CorePipelinePlugin::default())
        .add_plugin(bevy::pbr::PbrPlugin::default());

    // Initialise 3rd party bevy plugins
    app.add_plugin(NoCameraPlayerPlugin)
        .add_plugin(PolylinePlugin)
        .add_plugin(PickingPlugin)
        .add_plugin(InteractablePickingPlugin)
        .add_plugin(DebugCursorPickingPlugin)
        .add_system(control_picking)
        .add_system(picking_events);

    // Initialise rose stuff
    app.insert_resource(ClientConfiguration { zone_id })
        .init_asset_loader::<ZmsAssetLoader>()
        .add_plugin(RoseRenderPlugin)
        .add_startup_system(setup);

    app.run();
}

#[derive(Component)]
pub struct ZscMaterialComponent(ZscMaterial);

fn control_picking(windows: Res<Windows>, mut picking: ResMut<PickingPluginsState>) {
    let window = windows.get_primary().unwrap();
    let cursor_locked = window.cursor_locked();
    picking.enable_picking = !cursor_locked;
    picking.update_debug_cursor = !cursor_locked;
}

fn picking_events(
    mut commands: Commands,
    mut events: EventReader<PickingEvent>,
    mut polylines: ResMut<Assets<Polyline>>,
    mut polyline_materials: ResMut<Assets<PolylineMaterial>>,
    query: Query<(
        &Handle<Mesh>,
        &GlobalTransform,
        Option<&ZscMaterialComponent>,
    )>,
    existing_polylines: Query<Entity, With<Handle<Polyline>>>,
    meshes: Res<Assets<Mesh>>,
) {
    for event in events.iter() {
        if let &PickingEvent::Clicked(e) = event {
            if let Ok((mesh, &global_transform, zsc_material)) = query.get(e) {
                if let Some(mesh) = meshes.get(mesh) {
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

                        commands.spawn_bundle(PolylineBundle {
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
                        });

                        for existing in existing_polylines.iter() {
                            commands.entity(existing).despawn();
                        }
                    }
                }

                if let Some(zsc_material) = zsc_material {
                    println!("{:#?}", zsc_material.0);
                }
            }
        }
    }
}

#[derive(Component)]
pub struct LoadedZone;

#[allow(clippy::too_many_arguments)]
fn setup(
    mut commands: Commands,
    client_configuration: Res<ClientConfiguration>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut terrain_materials: ResMut<Assets<TerrainMaterial>>,
    mut static_mesh_materials: ResMut<Assets<StaticMeshMaterial>>,
    mut water_mesh_materials: ResMut<Assets<WaterMeshMaterial>>,
    mut texture_arrays: ResMut<Assets<TextureArray>>,
    vfs_resource: Res<VfsResource>,
) {
    // Create camera
    commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_xyz(5200.0, 0.0, -5200.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..Default::default()
        })
        .insert_bundle(PickingCameraBundle::default())
        .insert(FlyCam);

    commands
        .spawn_bundle((
            LoadedZone {},
            GlobalTransform::default(),
            Transform::default(),
        ))
        .with_children(|child_builder| {
            zone_loader::load_zone(
                child_builder,
                &asset_server,
                &vfs_resource,
                &mut meshes,
                &mut terrain_materials,
                &mut static_mesh_materials,
                &mut water_mesh_materials,
                &mut texture_arrays,
                client_configuration.zone_id,
            );
        });
}
