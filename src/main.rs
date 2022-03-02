mod material;
mod mesh_pipeline;
mod static_mesh_material;
mod terrain_material;
mod water_mesh_material;

use std::{
    io::Cursor,
    path::{Path, PathBuf},
};

use bevy::{
    asset::{AssetLoader, AssetServerSettings, BoxedFuture, LoadContext, LoadState, LoadedAsset},
    math::{Quat, Vec2, Vec3},
    prelude::{
        AddAsset, App, AssetServer, Assets, BuildChildren, Commands, ComputedVisibility,
        GlobalTransform, Handle, Image, Mesh, Msaa, PerspectiveCameraBundle, Res, ResMut, State,
        SystemSet, Transform, Visibility,
    },
    render::{
        mesh::Indices,
        render_resource::{Extent3d, PrimitiveTopology, TextureDimension, TextureFormat},
    },
    window::WindowDescriptor,
};
mod bevy_flycam;
use bevy_flycam::{FlyCam, MovementSettings, NoCameraPlayerPlugin};

use roselib::{
    files::{ifo, lit, zon::ZoneTileRotation, HIM, IFO, LIT, STB, TIL, ZMS, ZON, ZSC},
    io::{PathRoseExt, RoseFile, RoseReader},
};

use material::{AlphaMode, MaterialPlugin};
use mesh_pipeline::MeshRenderPlugin;
use static_mesh_material::{
    StaticMeshMaterial, StaticMeshMaterialPlugin, STATIC_MESH_ATTRIBUTE_UV1,
    STATIC_MESH_ATTRIBUTE_UV2, STATIC_MESH_ATTRIBUTE_UV3, STATIC_MESH_ATTRIBUTE_UV4,
};
use terrain_material::{
    TerrainMaterial, TerrainMaterialPlugin, TERRAIN_MESH_ATTRIBUTE_TILE_INFO,
    TERRAIN_MESH_ATTRIBUTE_UV1,
};
use water_mesh_material::{WaterMeshMaterial, WaterMeshMaterialPlugin};

struct ClientConfiguration {
    data_path: PathBuf,
    zone_id: usize,
}

fn main() {
    let matches = clap::Command::new("bevy_rose")
        .arg(
            clap::Arg::new("data-path")
                .long("data-path")
                .help("Path to extracted rose data")
                .required(true)
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
        )
        .get_matches();
    let data_path = matches.value_of("data-path").unwrap();
    let zone_id = matches
        .value_of("zone")
        .and_then(|str| str.parse::<usize>().ok())
        .unwrap_or(2);
    let disable_vsync = matches.is_present("disable-vsync");

    let mut app = App::new();

    // Initialise bevy engine
    app.insert_resource(Msaa { samples: 4 })
        .insert_resource(AssetServerSettings {
            asset_folder: data_path.to_string(),
            watch_for_changes: false,
        })
        .insert_resource(MovementSettings {
            sensitivity: 0.00012,
            speed: 200.,
        })
        .insert_resource(WindowDescriptor {
            present_mode: if disable_vsync {
                bevy::window::PresentMode::Immediate
            } else {
                bevy::window::PresentMode::Fifo
            },
            ..Default::default()
        })
        .add_plugin(bevy::log::LogPlugin::default())
        .add_plugin(bevy::core::CorePlugin::default())
        .add_plugin(bevy::diagnostic::LogDiagnosticsPlugin::default())
        .add_plugin(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default())
        .add_plugin(bevy::transform::TransformPlugin::default())
        .add_plugin(bevy::diagnostic::DiagnosticsPlugin::default())
        .add_plugin(bevy::input::InputPlugin::default())
        .add_plugin(bevy::window::WindowPlugin::default())
        .add_plugin(bevy::asset::AssetPlugin::default())
        .add_plugin(bevy::scene::ScenePlugin::default())
        .add_plugin(bevy::winit::WinitPlugin::default())
        .add_plugin(bevy::render::RenderPlugin::default())
        .add_plugin(bevy::core_pipeline::CorePipelinePlugin::default());

    // Initialise 3rd party bevy plugins
    app.add_plugin(NoCameraPlayerPlugin);

    // Initialise rose stuff
    app.insert_resource(ClientConfiguration {
        data_path: PathBuf::from(data_path),
        zone_id,
    })
    .init_resource::<ZoneInfo>()
    .init_asset_loader::<ZmsMeshAssetLoader>()
    .add_plugin(MeshRenderPlugin)
    .add_plugin(TerrainMaterialPlugin)
    .add_plugin(MaterialPlugin::<TerrainMaterial>::default())
    .add_plugin(StaticMeshMaterialPlugin)
    .add_plugin(MaterialPlugin::<StaticMeshMaterial>::default())
    .add_plugin(WaterMeshMaterialPlugin)
    .add_state(AppState::Setup)
    .add_system_set(SystemSet::on_enter(AppState::Setup).with_system(load_zone_tiles))
    .add_system_set(SystemSet::on_update(AppState::Setup).with_system(check_zone_tile_textures))
    .add_system_set(SystemSet::on_enter(AppState::Finished).with_system(setup));

    app.run();
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum AppState {
    Setup,
    Finished,
}

#[derive(Default)]
struct ZoneInfo {
    zone: ZON,
    zsc_deco: ZSC,
    zsc_cnst: ZSC,
    zone_path: PathBuf,
    tile_image_handles: Vec<Handle<Image>>,
    water_image_handles: Vec<Handle<Image>>,
}

fn load_zone_tiles(
    mut zone_info: ResMut<ZoneInfo>,
    asset_server: Res<AssetServer>,
    client_configuration: Res<ClientConfiguration>,
) {
    let zone_id = client_configuration.zone_id;
    let list_zone = STB::from_path(
        &client_configuration
            .data_path
            .join("3DDATA/STB/LIST_ZONE.STB"),
    )
    .unwrap();
    let zon_file_path = PathBuf::from_rose_path(list_zone.value(zone_id, 2).unwrap());
    let zsc_deco_path = PathBuf::from_rose_path(list_zone.value(zone_id, 12).unwrap());
    let zsc_cnst_path = PathBuf::from_rose_path(list_zone.value(zone_id, 13).unwrap());

    zone_info.zsc_cnst =
        ZSC::from_path(&client_configuration.data_path.join(zsc_cnst_path)).unwrap();
    zone_info.zsc_deco =
        ZSC::from_path(&client_configuration.data_path.join(zsc_deco_path)).unwrap();
    zone_info.zone_path = zon_file_path.parent().unwrap().into();

    let zon = ZON::from_path(&client_configuration.data_path.join(zon_file_path)).unwrap();

    for path in zon.textures.iter() {
        if path.to_lowercase().ends_with(".dds") {
            zone_info.tile_image_handles.push(asset_server.load(path));
        }
    }

    for i in 1..=25 {
        zone_info
            .water_image_handles
            .push(asset_server.load(&format!("3DDATA/JUNON/WATER/OCEAN01_{:02}.DDS", i)));
    }

    zone_info.zone = zon;
}

fn check_zone_tile_textures(
    mut state: ResMut<State<AppState>>,
    zone_info: ResMut<ZoneInfo>,
    asset_server: Res<AssetServer>,
) {
    if matches!(
        asset_server
            .get_group_load_state(zone_info.tile_image_handles.iter().map(|handle| handle.id)),
        LoadState::Loaded
    ) && matches!(
        asset_server
            .get_group_load_state(zone_info.water_image_handles.iter().map(|handle| handle.id)),
        LoadState::Loaded
    ) {
        state.set(AppState::Finished).unwrap();
    }
}

#[derive(Default)]
pub struct ZmsMeshAssetLoader;

impl AssetLoader for ZmsMeshAssetLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            let mut zms = ZMS::new();
            let mut reader = RoseReader::new(Cursor::new(bytes));
            zms.read(&mut reader).unwrap();

            let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);

            let mut indices = Vec::new();
            for index in zms.indices.iter() {
                indices.push(index.x as u16);
                indices.push(index.y as u16);
                indices.push(index.z as u16);
            }
            mesh.set_indices(Some(Indices::U16(indices)));

            let mut position = Vec::new();
            let mut colors = Vec::new();
            let mut normals = Vec::new();
            let mut tangents = Vec::new();
            let mut uv1 = Vec::new();
            let mut uv2 = Vec::new();
            let mut uv3 = Vec::new();
            let mut uv4 = Vec::new();
            let mut bone_weights = Vec::new();
            let mut bone_indices = Vec::new();

            for vertex in zms.vertices.iter() {
                if zms.positions_enabled() {
                    position.push([vertex.position.x, vertex.position.z, -vertex.position.y]);
                }

                if zms.normals_enabled() {
                    normals.push([vertex.normal.x, vertex.normal.z, -vertex.normal.y]);
                }

                if zms.tangents_enabled() {
                    tangents.push([vertex.tangent.x, vertex.tangent.z, -vertex.tangent.y]);
                }

                if zms.colors_enabled() {
                    colors.push([
                        vertex.color.r,
                        vertex.color.g,
                        vertex.color.b,
                        vertex.color.a,
                    ]);
                }

                if zms.uv1_enabled() {
                    uv1.push([vertex.uv1.x, vertex.uv1.y]);
                }

                if zms.uv2_enabled() {
                    uv2.push([vertex.uv2.x, vertex.uv2.y]);
                }

                if zms.uv3_enabled() {
                    uv3.push([vertex.uv3.x, vertex.uv3.y]);
                }

                if zms.uv4_enabled() {
                    uv4.push([vertex.uv4.x, vertex.uv4.y]);
                }

                if zms.bones_enabled() {
                    bone_weights.push([
                        vertex.bone_weights.x,
                        vertex.bone_weights.y,
                        vertex.bone_weights.z,
                        vertex.bone_weights.w,
                    ]);
                    bone_indices.push(
                        (vertex.bone_indices.x as u32 & 0xFF)
                            | (vertex.bone_indices.y as u32 & 0xFF) << 8
                            | (vertex.bone_indices.z as u32 & 0xFF) << 16
                            | (vertex.bone_indices.w as u32 & 0xFF) << 24,
                    );
                }
            }

            if !position.is_empty() {
                mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, position);
            }

            if !normals.is_empty() {
                mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
            }

            if !colors.is_empty() {
                mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
            }

            if !tangents.is_empty() {
                mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, tangents);
            }

            if !bone_weights.is_empty() {
                mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, bone_weights);
            }

            if !bone_indices.is_empty() {
                mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_INDEX, bone_indices);
            }

            if !uv1.is_empty() {
                mesh.insert_attribute(STATIC_MESH_ATTRIBUTE_UV1, uv1);
            }

            if !uv2.is_empty() {
                mesh.insert_attribute(STATIC_MESH_ATTRIBUTE_UV2, uv2);
            }

            if !uv3.is_empty() {
                mesh.insert_attribute(STATIC_MESH_ATTRIBUTE_UV3, uv3);
            }

            if !uv4.is_empty() {
                mesh.insert_attribute(STATIC_MESH_ATTRIBUTE_UV4, uv4);
            }

            load_context.set_default_asset(LoadedAsset::new(mesh));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["zms"]
    }
}

#[derive(Default)]
pub struct CustomAssetLoader;

#[allow(clippy::too_many_arguments)]
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut terrain_materials: ResMut<Assets<TerrainMaterial>>,
    mut static_mesh_materials: ResMut<Assets<StaticMeshMaterial>>,
    mut water_mesh_materials: ResMut<Assets<WaterMeshMaterial>>,
    zone_info: ResMut<ZoneInfo>,
    mut textures: ResMut<Assets<Image>>,
    client_configuration: Res<ClientConfiguration>,
) {
    // Create camera
    commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_xyz(5200.0, 0.0, -5200.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..Default::default()
        })
        .insert(FlyCam);

    // Create tile array texture
    let tile_texture_size = textures
        .get(zone_info.tile_image_handles[0].clone())
        .map(|image| image.size())
        .unwrap();
    let tile_texture_width = tile_texture_size.x as u32;
    let tile_texture_height = tile_texture_size.y as u32;
    let mut zone_tile_array = Image::new(
        Extent3d {
            width: tile_texture_width,
            height: tile_texture_height,
            depth_or_array_layers: zone_info.tile_image_handles.len() as u32,
        },
        TextureDimension::D2,
        vec![
            0;
            4 * (tile_texture_width * tile_texture_height) as usize
                * zone_info.tile_image_handles.len()
        ],
        TextureFormat::Rgba8Unorm,
    );
    for (i, handle) in zone_info.tile_image_handles.iter().enumerate() {
        let image = textures.get(handle).unwrap();
        let begin = 4 * (tile_texture_width * tile_texture_height) as usize * i;
        let end = 4 * (tile_texture_width * tile_texture_height) as usize * (i + 1);
        zone_tile_array.data[begin..end].copy_from_slice(&image.data);
    }
    let tile_array_texture = textures.add(zone_tile_array);

    // Create water array texture
    let water_texture_size = textures
        .get(zone_info.water_image_handles[0].clone())
        .map(|image| image.size())
        .unwrap();
    let water_texture_width = water_texture_size.x as u32;
    let water_texture_height = water_texture_size.y as u32;
    let mut water_tile_array = Image::new(
        Extent3d {
            width: water_texture_width,
            height: water_texture_height,
            depth_or_array_layers: zone_info.water_image_handles.len() as u32,
        },
        TextureDimension::D2,
        vec![
            0;
            4 * (water_texture_width * water_texture_height) as usize
                * zone_info.water_image_handles.len()
        ],
        TextureFormat::Rgba8Unorm,
    );
    for (i, handle) in zone_info.water_image_handles.iter().enumerate() {
        let image = textures.get(handle).unwrap();
        let begin = 4 * (water_texture_width * water_texture_height) as usize * i;
        let end = 4 * (water_texture_width * water_texture_height) as usize * (i + 1);
        water_tile_array.data[begin..end].copy_from_slice(&image.data);
    }
    let water_array_texture = textures.add(water_tile_array);

    let water_material = water_mesh_materials.add(WaterMeshMaterial {
        water_texture_array: water_array_texture,
    });

    // Load the zone
    let zone_base_directory = client_configuration.data_path.join(&zone_info.zone_path);
    for map_y in 0..64u32 {
        for map_x in 0..64u32 {
            let tilemap =
                TIL::from_path(&zone_base_directory.join(format!("{}_{}.TIL", map_x, map_y)));
            let heightmap =
                HIM::from_path(&zone_base_directory.join(format!("{}_{}.HIM", map_x, map_y)));

            if let (Ok(heightmap), Ok(tilemap)) = (heightmap, tilemap) {
                let offset_x = 160.0 * map_x as f32;
                let offset_y = 160.0 * (65.0 - map_y as f32);

                let material = terrain_materials.add(TerrainMaterial {
                    lightmap_texture: asset_server.load(&format!(
                        "{}/{}_{}/{}_{}_PLANELIGHTINGMAP.DDS",
                        zone_info.zone_path.to_str().unwrap(),
                        map_x,
                        map_y,
                        map_x,
                        map_y
                    )),
                    tile_array_texture: tile_array_texture.clone(),
                });

                let mut positions = Vec::new();
                let mut uvs_lightmap = Vec::new();
                let mut uvs_tile = Vec::new();
                let mut indices = Vec::new();
                let mut tile_ids = Vec::new();

                for block_x in 0..16 {
                    for block_y in 0..16 {
                        let base_x = block_x as f32 * 4.0 * 2.5;
                        let base_y = block_y as f32 * 4.0 * 2.5;

                        let tile =
                            &zone_info.zone.tiles[tilemap.tiles[block_y][block_x].tile_id as usize];
                        let tile_layer1 = tile.layer1 + tile.offset1;
                        let tile_layer2 = tile.layer2 + tile.offset2;
                        let index_base = positions.len() as u16;
                        let tile_rotation = match tile.rotation {
                            ZoneTileRotation::FlipHorizontal => 2,
                            ZoneTileRotation::FlipVertical => 3,
                            ZoneTileRotation::Flip => 4,
                            ZoneTileRotation::Clockwise90 => 5,
                            ZoneTileRotation::CounterClockwise90 => 6,
                            _ => 0,
                        };

                        for y in 0..5 {
                            for x in 0..5 {
                                let heightmap_x = x + block_x * 4;
                                let heightmap_y = y + block_y * 4;
                                let height = heightmap.height(heightmap_x, heightmap_y) / 100.0;

                                positions.push([
                                    base_x + x as f32 * 2.5,
                                    height,
                                    base_y + y as f32 * 2.5,
                                ]);
                                uvs_tile.push([x as f32 / 4.0, y as f32 / 4.0]);
                                uvs_lightmap.push([
                                    (block_x as f32 * 4.0 + x as f32) / 64.0,
                                    (block_y as f32 * 4.0 + y as f32) / 64.0,
                                ]);
                                tile_ids.push([tile_layer1, tile_layer2, tile_rotation]);
                            }
                        }

                        for y in 0..(5 - 1) {
                            for x in 0..(5 - 1) {
                                let start = index_base + y * 5 + x;
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
                mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs_lightmap);
                mesh.insert_attribute(TERRAIN_MESH_ATTRIBUTE_UV1, uvs_tile);
                mesh.insert_attribute(TERRAIN_MESH_ATTRIBUTE_TILE_INFO, tile_ids);

                commands.spawn().insert_bundle((
                    meshes.add(mesh),
                    material.clone(),
                    Transform::from_xyz(offset_x, 0.0, -offset_y),
                    GlobalTransform::default(),
                    Visibility::default(),
                    ComputedVisibility::default(),
                ));
            }

            let ifo = IFO::from_path(&zone_base_directory.join(format!("{}_{}.IFO", map_x, map_y)));
            let buildings_lit = LIT::from_path(&zone_base_directory.join(format!(
                "{}_{}/LIGHTMAP/BUILDINGLIGHTMAPDATA.LIT",
                map_x, map_y
            )))
            .ok();
            let objects_lit = LIT::from_path(&zone_base_directory.join(format!(
                "{}_{}/LIGHTMAP/OBJECTLIGHTMAPDATA.LIT",
                map_x, map_y
            )))
            .ok();
            let lit_path = zone_base_directory.join(format!("{}_{}/LIGHTMAP", map_x, map_y));

            if let Ok(ifo) = ifo {
                for ocean in ifo.oceans.iter() {
                    for patch in ocean.patches.iter() {
                        let start = Vec3::new(
                            patch.start.x / 100.0,
                            patch.start.y / 100.0,
                            -patch.start.z / 100.0,
                        );
                        let end = Vec3::new(
                            patch.end.x / 100.0,
                            patch.end.y / 100.0,
                            -patch.end.z / 100.0,
                        );
                        let uv_x = (end.x - start.x) / (ocean.size / 100.0);
                        let uv_y = (end.z - start.z) / (ocean.size / 100.0);

                        let vertices = [
                            ([start.x, start.y, end.z], [uv_x, uv_y]),
                            ([start.x, start.y, start.z], [uv_x, 0.0]),
                            ([end.x, start.y, start.z], [0.0, 0.0]),
                            ([end.x, start.y, end.z], [0.0, uv_y]),
                        ];
                        let indices = Indices::U32(vec![0, 2, 1, 0, 3, 2]);

                        let mut positions = Vec::new();
                        let mut uvs = Vec::new();
                        for (position, uv) in &vertices {
                            positions.push(*position);
                            uvs.push(*uv);
                        }

                        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
                        mesh.set_indices(Some(indices));
                        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
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
                }

                for (object_id, object_instance) in ifo.buildings.iter().enumerate() {
                    let lit_object = buildings_lit.as_ref().and_then(|lit| {
                        lit.objects
                            .iter()
                            .find(|lit_object| lit_object.id as usize == object_id + 1)
                    });

                    spawn_zsc_object(
                        &mut commands,
                        asset_server.as_ref(),
                        static_mesh_materials.as_mut(),
                        &zone_info.zsc_cnst,
                        &lit_path,
                        lit_object,
                        object_instance,
                    );
                }

                for (object_id, object_instance) in ifo.objects.iter().enumerate() {
                    let lit_object = objects_lit.as_ref().and_then(|lit| {
                        lit.objects
                            .iter()
                            .find(|lit_object| lit_object.id as usize == object_id + 1)
                    });

                    spawn_zsc_object(
                        &mut commands,
                        asset_server.as_ref(),
                        static_mesh_materials.as_mut(),
                        &zone_info.zsc_deco,
                        &lit_path,
                        lit_object,
                        object_instance,
                    );
                }
            }
        }
    }
}

fn spawn_zsc_object(
    commands: &mut Commands,
    asset_server: &AssetServer,
    static_mesh_materials: &mut Assets<StaticMeshMaterial>,
    zsc: &ZSC,
    lit_path: &Path,
    lit_object: Option<&lit::LightmapObject>,
    object_instance: &ifo::ObjectData,
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
                    let handle = asset_server.load(zsc.meshes[mesh_id].clone());
                    mesh_cache.insert(mesh_id, Some(handle.clone()));
                    handle
                });
                let lit_part = lit_object.and_then(|lit_object| lit_object.parts.get(part_index));
                let lightmap_texture = lit_part.map(|lit_part| {
                    asset_server.load(lit_path.join(PathBuf::from_rose_path(&lit_part.filename)))
                });
                let (lightmap_uv_offset, lightmap_uv_scale) = lit_part
                    .map(|lit_part| {
                        let scale = 1.0 / lit_part.parts_per_width as f32;
                        (
                            Vec2::new(
                                (lit_part.part_position % lit_part.parts_per_width) as f32,
                                (lit_part.part_position / lit_part.parts_per_width) as f32,
                            ),
                            scale,
                        )
                    })
                    .unwrap_or((Vec2::new(0.0, 0.0), 1.0));

                let material_id = object_part.material_id as usize;
                let material = material_cache[material_id].clone().unwrap_or_else(|| {
                    let zsc_material = &zsc.materials[material_id];
                    let handle = static_mesh_materials.add(StaticMeshMaterial {
                        base_texture: asset_server.load(zsc_material.path.clone()),
                        lightmap_texture,
                        alpha_value: if zsc_material.alpha != 1.0 {
                            Some(zsc_material.alpha)
                        } else {
                            None
                        },
                        alpha_mode: if zsc_material.alpha_test_enabled {
                            AlphaMode::Mask(zsc_material.alpha_ref as f32 / 256.0)
                        } else if zsc_material.alpha_enabled {
                            AlphaMode::Blend
                        } else {
                            AlphaMode::Opaque
                        },
                        two_sided: zsc_material.two_sided,
                        z_write_enabled: zsc_material.z_write_enabled,
                        z_test_enabled: zsc_material.z_test_enabled,
                        lightmap_uv_offset,
                        lightmap_uv_scale,
                    });

                    /*
                    pub blend_mode: SceneBlendMode,
                    pub specular_enabled: bool,
                    pub glow_type: SceneGlowType,
                    pub glow_color: Color3,
                    */
                    material_cache.insert(material_id, Some(handle.clone()));
                    handle
                });

                parent.spawn_bundle((
                    mesh,
                    material,
                    part_transform,
                    GlobalTransform::default(),
                    Visibility::default(),
                    ComputedVisibility::default(),
                ));
            }
        });
}

#[macro_export]
macro_rules! load_internal_asset {
    ($app: ident, $handle: ident, $path_str: expr, $loader: expr) => {{
        let mut assets = $app
            .world
            .get_resource_mut::<bevy::asset::Assets<_>>()
            .unwrap();
        assets.set_untracked($handle, ($loader)(include_str!($path_str)));
    }};
}
