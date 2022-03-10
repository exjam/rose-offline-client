use bevy::{
    asset::AssetServerSettings,
    core_pipeline::ClearColor,
    math::Vec3,
    pbr::{AlphaMode, StandardMaterial},
    prelude::{
        shape, AddAsset, App, AssetServer, Assets, BuildChildren, Changed, Color, Commands,
        Component, DespawnRecursiveExt, Entity, EventReader, GlobalTransform, Handle, Mesh, Msaa,
        PerspectiveCameraBundle, Query, Res, ResMut, State, SystemSet, Transform, With,
    },
    render::{
        mesh::{Indices, VertexAttributeValues},
        render_resource::WgpuFeatures,
        settings::WgpuSettings,
    },
    window::{WindowDescriptor, Windows},
};
use bevy_egui::{egui, EguiContext, EguiPlugin};
use bevy_mod_picking::{
    DebugCursorPickingPlugin, InteractablePickingPlugin, PickingCameraBundle, PickingEvent,
    PickingPlugin, PickingPluginsState,
};
use bevy_polyline::{Polyline, PolylineBundle, PolylineMaterial, PolylinePlugin};
use std::{path::Path, sync::Arc, time::Duration};

use nalgebra::Point3;
use rose_data::{
    EquipmentIndex, EquipmentItem, ItemDatabase, ItemReference, ItemType, ZoneId, ZoneList,
};
use rose_file_readers::VfsIndex;
use rose_game_common::components::{CharacterGender, CharacterInfo, Equipment};

mod bevy_flycam;
mod character_model;
mod render;
mod vfs_asset_io;
mod zms_asset_loader;
mod zone_loader;

use bevy_flycam::{FlyCam, MovementSettings, NoCameraPlayerPlugin};
use character_model::{spawn_character_model, CharacterModel, CharacterModelList};
use render::{
    RoseRenderPlugin, StaticMeshMaterial, TerrainMaterial, TextureArray, WaterMeshMaterial,
};
use vfs_asset_io::VfsAssetIo;
use zms_asset_loader::ZmsAssetLoader;
use zone_loader::ZoneObject;

use crate::character_model::update_character_equipment;

struct LoadZoneId {
    zone_id: ZoneId,
}

pub struct VfsResource {
    vfs: Arc<VfsIndex>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum AppState {
    LoadingZone,
    InGame,
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
        .and_then(|str| str.parse::<u16>().ok())
        .and_then(ZoneId::new)
        .unwrap_or_else(|| ZoneId::new(2).unwrap());
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
            width: 1920.0,
            height: 1080.0,
            ..Default::default()
        })
        .insert_resource(ClearColor(Color::rgb(0.70, 0.90, 1.0)))
        .insert_resource(WgpuSettings {
            features: WgpuFeatures::TEXTURE_COMPRESSION_BC,
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
        .insert_resource(AssetServer::new(VfsAssetIo::new(vfs.clone()), task_pool))
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
        .add_plugin(DebugCursorPickingPlugin);

    // Initialise rose stuff
    app.insert_resource(LoadZoneId { zone_id })
        .init_asset_loader::<ZmsAssetLoader>()
        .add_plugin(RoseRenderPlugin)
        .insert_resource(
            CharacterModelList::new(&vfs).expect("Failed to load character model list"),
        );

    app.add_plugin(EguiPlugin).add_system(draw_debug_ui);

    // Setup state
    app.add_state(AppState::LoadingZone)
        .add_system_set(
            SystemSet::on_enter(AppState::LoadingZone).with_system(state_enter_load_zone),
        )
        .add_system_set(SystemSet::on_exit(AppState::InGame).with_system(state_leave_in_game))
        .add_system_set(
            SystemSet::on_update(AppState::InGame)
                .with_system(control_picking)
                .with_system(update_character_model)
                .with_system(picking_events),
        )
        .add_startup_system(setup);

    app.run();
}

fn control_picking(windows: Res<Windows>, mut picking: ResMut<PickingPluginsState>) {
    let window = windows.get_primary().unwrap();
    let cursor_locked = window.cursor_locked();
    picking.enable_picking = !cursor_locked;
    picking.update_debug_cursor = !cursor_locked;
}

#[allow(clippy::too_many_arguments)]
fn picking_events(
    mut commands: Commands,
    mut events: EventReader<PickingEvent>,
    mut polylines: ResMut<Assets<Polyline>>,
    mut polyline_materials: ResMut<Assets<PolylineMaterial>>,
    mut debug_ui_state: ResMut<DebugUiState>,
    query: Query<(&Handle<Mesh>, &GlobalTransform)>,
    query_loaded_zone: Query<Entity, With<LoadedZone>>,
    existing_polylines: Query<Entity, With<Handle<Polyline>>>,
    meshes: Res<Assets<Mesh>>,
) {
    let loaded_zone = query_loaded_zone.single();

    for event in events.iter() {
        if let &PickingEvent::Clicked(entity) = event {
            if let Ok((mesh, &global_transform)) = query.get(entity) {
                if let Some(mesh) = meshes.get(mesh) {
                    debug_ui_state.inspecting_entity = Some(entity);

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

                        commands.entity(loaded_zone).with_children(|parent| {
                            parent.spawn_bundle(PolylineBundle {
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
                        });

                        for existing in existing_polylines.iter() {
                            commands.entity(existing).despawn();
                        }
                    }
                }
            }
        }
    }
}

#[derive(Component)]
pub struct LoadedZone;

fn setup(
    mut commands: Commands,
    vfs_resource: Res<VfsResource>,
    asset_server: Res<AssetServer>,
    character_model_list: Res<CharacterModelList>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut static_mesh_materials: ResMut<Assets<StaticMeshMaterial>>,
) {
    // Create camera
    commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_xyz(5200.0, 0.0, -5200.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..Default::default()
        })
        .insert_bundle(PickingCameraBundle::default())
        .insert(FlyCam);

    // Load list of zones
    commands.insert_resource(
        rose_data_irose::get_zone_list(&vfs_resource.vfs).expect("Failed to load zone list"),
    );

    commands.insert_resource(DebugUiState {
        zone_list_open: true,
        inspecting_entity: None,
        item_list_open: true,
    });
    commands.insert_resource(DebugUiItemListState {
        item_list_type: ItemType::Face,
    });

    commands.insert_resource(
        rose_data_irose::get_item_database(&vfs_resource.vfs)
            .expect("Failed to load item database"),
    );

    // Create a character
    let bone_mesh = meshes.add(Mesh::from(shape::Cube { size: 0.1 }));
    let bone_material = materials.add(StandardMaterial {
        base_color: Color::rgba(1.0, 0.08, 0.58, 0.75),
        alpha_mode: AlphaMode::Blend,
        ..Default::default()
    });

    let character_info = CharacterInfo {
        name: "Bot 1".into(),
        gender: CharacterGender::Male,
        race: 0,
        face: 8,
        hair: 10,
        birth_stone: 0,
        job: 0,
        rank: 0,
        fame: 0,
        fame_b: 0,
        fame_g: 0,
        revive_zone_id: ZoneId::new(22).unwrap(),
        revive_position: Point3::new(5200.0, 0.0, -5200.0),
        unique_id: 0,
    };
    let mut equipment = Equipment::default();
    equipment
        .equip_item(EquipmentItem::new(&ItemReference::new(ItemType::Weapon, 1)).unwrap())
        .ok();
    equipment
        .equip_item(EquipmentItem::new(&ItemReference::new(ItemType::SubWeapon, 1)).unwrap())
        .ok();

    let character_model = spawn_character_model(
        &mut commands,
        &asset_server,
        &mut static_mesh_materials,
        &character_model_list,
        &character_info,
        &equipment,
        Some((bone_mesh, bone_material)),
    );
    let root_bone = character_model.skeleton.bones[0];
    commands
        .spawn_bundle((
            character_info,
            equipment,
            character_model,
            GlobalTransform::default(),
            Transform::from_translation(Vec3::new(5200.0, 20.0, -5200.0))
                .with_scale(Vec3::new(10.0, 10.0, 10.0)),
        ))
        .add_child(root_bone);
}

fn update_character_model(
    mut commands: Commands,
    mut query: Query<(&CharacterInfo, &mut CharacterModel, &Equipment), Changed<Equipment>>,
    asset_server: Res<AssetServer>,
    character_model_list: Res<CharacterModelList>,
    mut static_mesh_materials: ResMut<Assets<StaticMeshMaterial>>,
) {
    for (character_info, mut character_model, equipment) in query.iter_mut() {
        println!("Equipment changed");
        update_character_equipment(
            &mut commands,
            &asset_server,
            &mut static_mesh_materials,
            &character_model_list,
            &mut character_model,
            character_info,
            equipment,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn state_enter_load_zone(
    mut commands: Commands,
    load_zone_id: Res<LoadZoneId>,
    asset_server: Res<AssetServer>,
    vfs_resource: Res<VfsResource>,
    zone_list: Res<ZoneList>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut terrain_materials: ResMut<Assets<TerrainMaterial>>,
    mut static_mesh_materials: ResMut<Assets<StaticMeshMaterial>>,
    mut water_mesh_materials: ResMut<Assets<WaterMeshMaterial>>,
    mut texture_arrays: ResMut<Assets<TextureArray>>,
    mut state: ResMut<State<AppState>>,
) {
    commands
        .spawn_bundle((
            LoadedZone {},
            GlobalTransform::default(),
            Transform::default(),
        ))
        .with_children(|child_builder| {
            if let Some(zone_list_entry) = zone_list.get_zone(load_zone_id.zone_id) {
                zone_loader::load_zone(
                    child_builder,
                    &asset_server,
                    &vfs_resource,
                    &mut meshes,
                    &mut terrain_materials,
                    &mut static_mesh_materials,
                    &mut water_mesh_materials,
                    &mut texture_arrays,
                    zone_list_entry,
                );
            }
        });

    state.set(AppState::InGame).unwrap();
}

fn state_leave_in_game(mut commands: Commands, loaded_zone_query: Query<Entity, With<LoadedZone>>) {
    commands
        .entity(loaded_zone_query.single())
        .despawn_recursive();
}

pub struct DebugUiState {
    zone_list_open: bool,
    inspecting_entity: Option<Entity>,
    item_list_open: bool,
}

pub struct DebugUiItemListState {
    item_list_type: ItemType,
}

#[allow(clippy::too_many_arguments)]
fn draw_debug_ui(
    mut egui_context: ResMut<EguiContext>,
    mut state: ResMut<State<AppState>>,
    zone_list: Res<ZoneList>,
    item_database: Res<ItemDatabase>,
    mut debug_ui_state: ResMut<DebugUiState>,
    mut debug_item_list_state: ResMut<DebugUiItemListState>,
    mut load_zone_id: ResMut<LoadZoneId>,
    query_zone_object: Query<&ZoneObject>,
    mut query_character: Query<(&mut Equipment,)>,
) {
    egui::Window::new("Zone List")
        .vscroll(true)
        .resizable(true)
        .default_height(300.0)
        .open(&mut debug_ui_state.zone_list_open)
        .show(egui_context.ctx_mut(), |ui| {
            egui::Grid::new("zone_list_grid").show(ui, |ui| {
                ui.label("id");
                ui.label("name");
                ui.end_row();

                for zone in zone_list.iter() {
                    ui.label(format!("{}", zone.id.get()));
                    ui.label(&zone.name);
                    if ui.button("Load").clicked() {
                        load_zone_id.zone_id = zone.id;
                        state.set(AppState::LoadingZone).unwrap();
                    }
                    ui.end_row();
                }
            });
        });

    if let Some(zone_object) = debug_ui_state
        .inspecting_entity
        .and_then(|entity| query_zone_object.get(entity).ok())
    {
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

    egui::Window::new("Item List")
        .vscroll(true)
        .resizable(true)
        .default_height(300.0)
        .open(&mut debug_ui_state.item_list_open)
        .show(egui_context.ctx_mut(), |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut debug_item_list_state.item_list_type,
                    ItemType::Face,
                    "Face",
                );
                ui.selectable_value(
                    &mut debug_item_list_state.item_list_type,
                    ItemType::Head,
                    "Head",
                );
                ui.selectable_value(
                    &mut debug_item_list_state.item_list_type,
                    ItemType::Body,
                    "Body",
                );
                ui.selectable_value(
                    &mut debug_item_list_state.item_list_type,
                    ItemType::Hands,
                    "Hands",
                );
                ui.selectable_value(
                    &mut debug_item_list_state.item_list_type,
                    ItemType::Feet,
                    "Feet",
                );
                ui.selectable_value(
                    &mut debug_item_list_state.item_list_type,
                    ItemType::Back,
                    "Back",
                );
                ui.selectable_value(
                    &mut debug_item_list_state.item_list_type,
                    ItemType::Weapon,
                    "Weapon",
                );
                ui.selectable_value(
                    &mut debug_item_list_state.item_list_type,
                    ItemType::SubWeapon,
                    "SubWeapon",
                );
            });
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut debug_item_list_state.item_list_type,
                    ItemType::Jewellery,
                    "Jewellery",
                );
                ui.selectable_value(
                    &mut debug_item_list_state.item_list_type,
                    ItemType::Consumable,
                    "Consumable",
                );
                ui.selectable_value(
                    &mut debug_item_list_state.item_list_type,
                    ItemType::Gem,
                    "Gem",
                );
                ui.selectable_value(
                    &mut debug_item_list_state.item_list_type,
                    ItemType::Material,
                    "Material",
                );
                ui.selectable_value(
                    &mut debug_item_list_state.item_list_type,
                    ItemType::Quest,
                    "Quest",
                );
                ui.selectable_value(
                    &mut debug_item_list_state.item_list_type,
                    ItemType::Vehicle,
                    "Vehicle",
                );
            });

            egui::Grid::new("item_list_grid").show(ui, |ui| {
                ui.label("id");
                ui.label("name");
                ui.end_row();

                for item_reference in item_database.iter_items(debug_item_list_state.item_list_type)
                {
                    if let Some(item_data) = item_database.get_base_item(item_reference) {
                        if !item_data.name.is_empty() {
                            ui.label(format!("{}", item_reference.item_number));
                            ui.label(&item_data.name);

                            if item_reference.item_type.is_equipment_item()
                                && ui.button("Equip").clicked()
                            {
                                for (mut equipment,) in query_character.iter_mut() {
                                    match item_reference.item_type {
                                        ItemType::Face => {
                                            equipment.equipped_items[EquipmentIndex::Face] =
                                                Some(EquipmentItem::new(&item_reference).unwrap())
                                        }
                                        ItemType::Head => {
                                            equipment.equipped_items[EquipmentIndex::Head] =
                                                Some(EquipmentItem::new(&item_reference).unwrap())
                                        }
                                        ItemType::Body => {
                                            equipment.equipped_items[EquipmentIndex::Body] =
                                                Some(EquipmentItem::new(&item_reference).unwrap())
                                        }
                                        ItemType::Hands => {
                                            equipment.equipped_items[EquipmentIndex::Hands] =
                                                Some(EquipmentItem::new(&item_reference).unwrap())
                                        }
                                        ItemType::Feet => {
                                            equipment.equipped_items[EquipmentIndex::Feet] =
                                                Some(EquipmentItem::new(&item_reference).unwrap())
                                        }
                                        ItemType::Back => {
                                            equipment.equipped_items[EquipmentIndex::Back] =
                                                Some(EquipmentItem::new(&item_reference).unwrap())
                                        }
                                        ItemType::Weapon => {
                                            equipment.equipped_items[EquipmentIndex::WeaponRight] =
                                                Some(EquipmentItem::new(&item_reference).unwrap())
                                        }
                                        ItemType::SubWeapon => {
                                            equipment.equipped_items[EquipmentIndex::WeaponLeft] =
                                                Some(EquipmentItem::new(&item_reference).unwrap())
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            ui.end_row();
                        }
                    }
                }
            });
        });
}
