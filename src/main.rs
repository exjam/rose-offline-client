mod render;
mod vfs_asset_io;
mod zms_asset_loader;

use std::{path::Path, sync::Arc, time::Duration};

use bevy::{
    asset::AssetServerSettings,
    math::{Quat, Vec2, Vec3},
    prelude::{
        AddAsset, App, AssetServer, Assets, BuildChildren, Color, Commands, Component,
        ComputedVisibility, Entity, EventReader, GlobalTransform, Handle, Mesh, Msaa,
        PerspectiveCameraBundle, Query, Res, ResMut, Transform, Visibility, With,
    },
    render::{
        mesh::{Indices, VertexAttributeValues},
        render_resource::PrimitiveTopology,
    },
    window::{WindowDescriptor, Windows},
};
mod bevy_flycam;
use bevy_flycam::{FlyCam, MovementSettings, NoCameraPlayerPlugin};
use bevy_mod_picking::{
    DebugCursorPickingPlugin, InteractablePickingPlugin, PickableBundle, PickingCameraBundle,
    PickingEvent, PickingPlugin, PickingPluginsState,
};

use bevy_polyline::{Polyline, PolylineBundle, PolylineMaterial, PolylinePlugin};
use render::{
    RoseRenderPlugin, StaticMeshMaterial, TerrainMaterial, TextureArray, TextureArrayBuilder,
    WaterMeshMaterial, MESH_ATTRIBUTE_UV_1, TERRAIN_MESH_ATTRIBUTE_TILE_INFO,
};
use rose_file_readers::{
    HimFile, IfoFile, IfoObject, LitFile, LitObject, StbFile, TilFile, VfsIndex, VfsPath, ZonFile,
    ZonTileRotation, ZscFile, ZscMaterial,
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

    // Load zone info
    let zone_id = client_configuration.zone_id;
    let list_zone = vfs_resource
        .vfs
        .read_file::<StbFile, _>("3DDATA/STB/LIST_ZONE.STB")
        .unwrap();
    let zon_file_path = VfsPath::from(list_zone.get(zone_id, 1));
    let zsc_deco_path = VfsPath::from(list_zone.get(zone_id, 11));
    let zsc_cnst_path = VfsPath::from(list_zone.get(zone_id, 12));

    let zsc_cnst = vfs_resource
        .vfs
        .read_file::<ZscFile, _>(&zsc_cnst_path)
        .ok();
    let zsc_deco = vfs_resource
        .vfs
        .read_file::<ZscFile, _>(&zsc_deco_path)
        .ok();
    let zone_path = zon_file_path.path().parent().unwrap();

    let zone_file = vfs_resource
        .vfs
        .read_file::<ZonFile, _>(&zon_file_path)
        .unwrap();

    // Load zone tile array
    let mut tile_texture_array_builder = TextureArrayBuilder::new();
    for path in zone_file.tile_textures.iter() {
        if path == "end" {
            break;
        }

        tile_texture_array_builder.add(path.clone());
    }
    let tile_texture_array = texture_arrays.add(tile_texture_array_builder.build(&asset_server));

    // Load zone water array
    let mut water_texture_array_builder = TextureArrayBuilder::new();
    for i in 1..=25 {
        water_texture_array_builder.add(format!("3DDATA/JUNON/WATER/OCEAN01_{:02}.DDS", i));
    }
    let water_material = water_mesh_materials.add(WaterMeshMaterial {
        water_texture_array: texture_arrays.add(water_texture_array_builder.build(&asset_server)),
    });

    // Load the zone
    for block_y in 0..64u32 {
        for block_x in 0..64u32 {
            let tilemap = vfs_resource
                .vfs
                .read_file::<TilFile, _>(zone_path.join(format!("{}_{}.TIL", block_x, block_y)));
            let heightmap = vfs_resource
                .vfs
                .read_file::<HimFile, _>(zone_path.join(format!("{}_{}.HIM", block_x, block_y)));

            if let (Ok(heightmap), Ok(tilemap)) = (heightmap, tilemap) {
                let offset_x = 160.0 * block_x as f32;
                let offset_y = 160.0 * (65.0 - block_y as f32);

                let material = terrain_materials.add(TerrainMaterial {
                    lightmap_texture: asset_server.load(&format!(
                        "{}/{1:}_{2:}/{1:}_{2:}_PLANELIGHTINGMAP.DDS",
                        zone_path.to_str().unwrap(),
                        block_x,
                        block_y,
                    )),
                    tile_array_texture: tile_texture_array.clone(),
                });

                let mut positions = Vec::new();
                let mut normals = Vec::new();
                let mut uvs_lightmap = Vec::new();
                let mut uvs_tile = Vec::new();
                let mut indices = Vec::new();
                let mut tile_ids = Vec::new();

                for tile_x in 0..16 {
                    for tile_y in 0..16 {
                        let tile = &zone_file.tiles[tilemap.get_clamped(tile_x, tile_y) as usize];
                        let tile_array_index1 = tile.layer1 + tile.offset1;
                        let tile_array_index2 = tile.layer2 + tile.offset2;
                        let tile_rotation = match tile.rotation {
                            ZonTileRotation::FlipHorizontal => 2,
                            ZonTileRotation::FlipVertical => 3,
                            ZonTileRotation::Flip => 4,
                            ZonTileRotation::Clockwise90 => 5,
                            ZonTileRotation::CounterClockwise90 => 6,
                            _ => 0,
                        };
                        let tile_indices_base = positions.len() as u16;
                        let tile_offset_x = tile_x as f32 * 4.0 * 2.5;
                        let tile_offset_y = tile_y as f32 * 4.0 * 2.5;

                        for y in 0..5 {
                            for x in 0..5 {
                                let heightmap_x = x + tile_x as i32 * 4;
                                let heightmap_y = y + tile_y as i32 * 4;
                                let height =
                                    heightmap.get_clamped(heightmap_x, heightmap_y) / 100.0;
                                let height_l =
                                    heightmap.get_clamped(heightmap_x - 1, heightmap_y) / 100.0;
                                let height_r =
                                    heightmap.get_clamped(heightmap_x + 1, heightmap_y) / 100.0;
                                let height_t =
                                    heightmap.get_clamped(heightmap_x, heightmap_y - 1) / 100.0;
                                let height_b =
                                    heightmap.get_clamped(heightmap_x, heightmap_y + 1) / 100.0;
                                let normal = Vec3::new(
                                    (height_l - height_r) / 2.0,
                                    1.0,
                                    (height_t - height_b) / 2.0,
                                )
                                .normalize();

                                positions.push([
                                    tile_offset_x + x as f32 * 2.5,
                                    height,
                                    tile_offset_y + y as f32 * 2.5,
                                ]);
                                normals.push([normal.x, normal.y, normal.z]);
                                uvs_tile.push([x as f32 / 4.0, y as f32 / 4.0]);
                                uvs_lightmap.push([
                                    (tile_x as f32 * 4.0 + x as f32) / 64.0,
                                    (tile_y as f32 * 4.0 + y as f32) / 64.0,
                                ]);
                                tile_ids.push([
                                    tile_array_index1,
                                    tile_array_index2,
                                    tile_rotation,
                                ]);
                            }
                        }

                        for y in 0..(5 - 1) {
                            for x in 0..(5 - 1) {
                                let start = tile_indices_base + y * 5 + x;
                                indices.push(start);
                                indices.push(start + 5);
                                indices.push(start + 1);

                                indices.push(start + 1);
                                indices.push(start + 5);
                                indices.push(start + 1 + 5);
                            }
                        }
                    }
                }

                let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
                mesh.set_indices(Some(Indices::U16(indices)));
                mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
                mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
                mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs_lightmap);
                mesh.insert_attribute(MESH_ATTRIBUTE_UV_1, uvs_tile);
                mesh.insert_attribute(TERRAIN_MESH_ATTRIBUTE_TILE_INFO, tile_ids);

                commands
                    .spawn()
                    .insert_bundle((
                        meshes.add(mesh),
                        material.clone(),
                        Transform::from_xyz(offset_x, 0.0, -offset_y),
                        GlobalTransform::default(),
                        Visibility::default(),
                        ComputedVisibility::default(),
                    ))
                    .insert_bundle(PickableBundle::default());
            }

            let ifo = vfs_resource
                .vfs
                .read_file::<IfoFile, _>(zone_path.join(format!("{}_{}.IFO", block_x, block_y)));
            let cnst_lit = vfs_resource
                .vfs
                .read_file::<LitFile, _>(zone_path.join(format!(
                    "{}_{}/LIGHTMAP/BUILDINGLIGHTMAPDATA.LIT",
                    block_x, block_y
                )))
                .ok();
            let deco_lit = vfs_resource
                .vfs
                .read_file::<LitFile, _>(zone_path.join(format!(
                    "{}_{}/LIGHTMAP/OBJECTLIGHTMAPDATA.LIT",
                    block_x, block_y
                )))
                .ok();

            let lightmap_path = zone_path.join(format!("{}_{}/LIGHTMAP/", block_x, block_y));

            if let Ok(ifo) = ifo {
                for (plane_start, plane_end) in ifo.water_planes.iter() {
                    let start = Vec3::new(
                        plane_start.x / 100.0,
                        plane_start.y / 100.0,
                        -plane_start.z / 100.0,
                    );
                    let end = Vec3::new(
                        plane_end.x / 100.0,
                        plane_end.y / 100.0,
                        -plane_end.z / 100.0,
                    );
                    let uv_x = (end.x - start.x) / (ifo.water_size / 100.0);
                    let uv_y = (end.z - start.z) / (ifo.water_size / 100.0);

                    let vertices = [
                        ([start.x, start.y, end.z], [0.0, 1.0, 0.0], [uv_x, uv_y]),
                        ([start.x, start.y, start.z], [0.0, 1.0, 0.0], [uv_x, 0.0]),
                        ([end.x, start.y, start.z], [0.0, 1.0, 0.0], [0.0, 0.0]),
                        ([end.x, start.y, end.z], [0.0, 1.0, 0.0], [0.0, uv_y]),
                    ];
                    let indices = Indices::U32(vec![0, 2, 1, 0, 3, 2]);

                    let mut positions = Vec::new();
                    let mut normals = Vec::new();
                    let mut uvs = Vec::new();
                    for (position, normal, uv) in &vertices {
                        positions.push(*position);
                        normals.push(*normal);
                        uvs.push(*uv);
                    }

                    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
                    mesh.set_indices(Some(indices));
                    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
                    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
                    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

                    commands.spawn().insert_bundle((
                        meshes.add(mesh),
                        water_material.clone(),
                        Transform::from_xyz(5200.0, 0.0, -5200.0),
                        GlobalTransform::default(),
                        Visibility::default(),
                        ComputedVisibility::default(),
                    ));
                }

                if let Some(zsc_cnst) = zsc_cnst.as_ref() {
                    for (object_id, object_instance) in ifo.cnst_objects.iter().enumerate() {
                        let lit_object = cnst_lit.as_ref().and_then(|lit| {
                            lit.objects
                                .iter()
                                .find(|lit_object| lit_object.id as usize == object_id + 1)
                        });

                        spawn_zsc_object(
                            &mut commands,
                            asset_server.as_ref(),
                            static_mesh_materials.as_mut(),
                            zsc_cnst,
                            &lightmap_path,
                            lit_object,
                            object_instance,
                        );
                    }
                }

                if let Some(zsc_deco) = zsc_deco.as_ref() {
                    for (object_id, object_instance) in ifo.deco_objects.iter().enumerate() {
                        let lit_object = deco_lit.as_ref().and_then(|lit| {
                            lit.objects
                                .iter()
                                .find(|lit_object| lit_object.id as usize == object_id + 1)
                        });

                        spawn_zsc_object(
                            &mut commands,
                            asset_server.as_ref(),
                            static_mesh_materials.as_mut(),
                            zsc_deco,
                            &lightmap_path,
                            lit_object,
                            object_instance,
                        );
                    }
                }
            }
        }
    }
}

fn spawn_zsc_object(
    commands: &mut Commands,
    asset_server: &AssetServer,
    static_mesh_materials: &mut Assets<StaticMeshMaterial>,
    zsc: &ZscFile,
    lightmap_path: &Path,
    lit_object: Option<&LitObject>,
    object_instance: &IfoObject,
) {
    let object = &zsc.objects[object_instance.object_id as usize];

    let position = Vec3::new(
        object_instance.position.x,
        object_instance.position.z,
        -object_instance.position.y,
    ) / 100.0
        + Vec3::new(5200.0, 0.0, -5200.0);

    let scale = Vec3::new(
        object_instance.scale.x,
        object_instance.scale.z,
        object_instance.scale.y,
    );

    let rotation = Quat::from_xyzw(
        object_instance.rotation.x,
        object_instance.rotation.z,
        -object_instance.rotation.y,
        object_instance.rotation.w,
    );

    let transform = Transform::default()
        .with_translation(position)
        .with_rotation(rotation)
        .with_scale(scale);

    let mut material_cache: Vec<Option<Handle<StaticMeshMaterial>>> =
        vec![None; zsc.materials.len()];
    let mut mesh_cache: Vec<Option<Handle<Mesh>>> = vec![None; zsc.meshes.len()];

    commands
        .spawn_bundle((
            transform,
            GlobalTransform::default(),
            Visibility::default(),
            ComputedVisibility::default(),
        ))
        .with_children(|parent| {
            for (part_index, object_part) in object.parts.iter().enumerate() {
                let part_position = Vec3::new(
                    object_part.position.x,
                    object_part.position.z,
                    -object_part.position.y,
                ) / 100.0;
                let part_scale = Vec3::new(
                    object_part.scale.x,
                    object_part.scale.z,
                    object_part.scale.y,
                );
                let part_rotation = Quat::from_xyzw(
                    object_part.rotation.x,
                    object_part.rotation.z,
                    -object_part.rotation.y,
                    object_part.rotation.w,
                );
                let part_transform = Transform::default()
                    .with_translation(part_position)
                    .with_rotation(part_rotation)
                    .with_scale(part_scale);

                let mesh_id = object_part.mesh_id as usize;
                let mesh = mesh_cache[mesh_id].clone().unwrap_or_else(|| {
                    let handle = asset_server.load(zsc.meshes[mesh_id].path());
                    mesh_cache.insert(mesh_id, Some(handle.clone()));
                    handle
                });
                let lit_part = lit_object.and_then(|lit_object| lit_object.parts.get(part_index));
                let lightmap_texture = lit_part
                    .map(|lit_part| asset_server.load(lightmap_path.join(&lit_part.filename)));
                let (lightmap_uv_offset, lightmap_uv_scale) = lit_part
                    .map(|lit_part| {
                        let scale = 1.0 / lit_part.parts_per_row as f32;
                        (
                            Vec2::new(
                                (lit_part.part_index % lit_part.parts_per_row) as f32,
                                (lit_part.part_index / lit_part.parts_per_row) as f32,
                            ),
                            scale,
                        )
                    })
                    .unwrap_or((Vec2::new(0.0, 0.0), 1.0));

                let material_id = object_part.material_id as usize;
                let material = material_cache[material_id].clone().unwrap_or_else(|| {
                    let zsc_material = &zsc.materials[material_id];
                    let handle = static_mesh_materials.add(StaticMeshMaterial {
                        base_texture: asset_server.load(zsc_material.path.path()),
                        lightmap_texture,
                        alpha_value: if zsc_material.alpha != 1.0 {
                            Some(zsc_material.alpha)
                        } else {
                            None
                        },
                        alpha_enabled: zsc_material.alpha_enabled,
                        alpha_test: zsc_material.alpha_test,
                        two_sided: zsc_material.two_sided,
                        z_write_enabled: zsc_material.z_write_enabled,
                        z_test_enabled: zsc_material.z_test_enabled,
                        lightmap_uv_offset,
                        lightmap_uv_scale,
                    });

                    /*
                    pub blend_mode: SceneBlendMode,
                    pub specular_enabled: bool,
                    pub glow: Option<ZscMaterialGlow>,
                    */
                    material_cache.insert(material_id, Some(handle.clone()));
                    handle
                });

                parent
                    .spawn_bundle((
                        mesh,
                        material,
                        part_transform,
                        GlobalTransform::default(),
                        Visibility::default(),
                        ComputedVisibility::default(),
                        ZscMaterialComponent(zsc.materials[material_id].clone()),
                    ))
                    .insert_bundle(PickableBundle::default());
            }
        });
}
