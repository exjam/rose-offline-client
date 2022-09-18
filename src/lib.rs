#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

use bevy::{
    asset::AssetServerSettings,
    core_pipeline::clear_color::ClearColor,
    ecs::{event::Events, schedule::ShouldRun},
    log::{Level, LogSettings},
    pbr::AmbientLight,
    prelude::{
        AddAsset, App, AssetServer, Assets, Camera3dBundle, Color, Commands, CoreStage,
        ExclusiveSystemDescriptorCoercion, IntoExclusiveSystem, Msaa,
        ParallelSystemDescriptorCoercion, Quat, Res, ResMut, StageLabel, StartupStage, State,
        SystemSet, SystemStage, Transform, Vec3,
    },
    render::{render_resource::WgpuFeatures, settings::WgpuSettings},
    window::{WindowDescriptor, WindowMode},
};
use bevy_egui::{egui, EguiContext};
use bevy_rapier3d::plugin::PhysicsStages;
use enum_map::enum_map;
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use rose_data::{CharacterMotionDatabaseOptions, NpcDatabaseOptions, ZoneId};
use rose_file_readers::{
    AruaVfsIndex, HostFilesystemDevice, LtbFile, StbFile, TitanVfsIndex, VfsIndex,
    VirtualFilesystem, VirtualFilesystemDevice, ZscFile,
};

pub mod audio;
pub mod bundles;
pub mod components;
pub mod effect_loader;
pub mod events;
pub mod free_camera;
pub mod model_loader;
pub mod orbit_camera;
pub mod protocol;
pub mod ray_from_screenspace;
pub mod render;
pub mod resources;
pub mod scripting;
pub mod systems;
pub mod ui;
pub mod vfs_asset_io;
pub mod zmo_asset_loader;
pub mod zms_asset_loader;
pub mod zone_loader;

use audio::OddioPlugin;
use events::{
    AnimationFrameEvent, BankEvent, CharacterSelectEvent, ChatboxEvent, ClientEntityEvent,
    ConversationDialogEvent, GameConnectionEvent, HitEvent, LoadZoneEvent, LoginEvent,
    MessageBoxEvent, NetworkEvent, NpcStoreEvent, NumberInputDialogEvent, PartyEvent,
    PersonalStoreEvent, PlayerCommandEvent, QuestTriggerEvent, SpawnEffectEvent,
    SpawnProjectileEvent, SystemFuncEvent, WorldConnectionEvent, ZoneEvent,
};
use free_camera::FreeCameraPlugin;
use model_loader::ModelLoader;
use orbit_camera::OrbitCameraPlugin;
use render::{DamageDigitMaterial, RoseRenderPlugin};
use resources::{
    load_ui_resources, run_network_thread, update_ui_resources, AppState, ClientEntityList,
    DamageDigitsSpawner, DebugRenderConfig, GameData, NameTagSettings, NetworkThread,
    NetworkThreadMessage, RenderConfiguration, SelectedTarget, ServerConfiguration, SoundSettings,
    VfsResource, WorldTime, ZoneTime,
};
use scripting::RoseScriptingPlugin;
use systems::{
    ability_values_system, animation_effect_system, animation_sound_system, animation_system,
    auto_login_system, background_music_system, character_model_add_collider_system,
    character_model_blink_system, character_model_update_system, character_select_enter_system,
    character_select_event_system, character_select_exit_system, character_select_input_system,
    character_select_models_system, character_select_system, client_entity_event_system,
    collision_height_only_system, collision_player_system, collision_player_system_join_zoin,
    command_system, conversation_dialog_system, cooldown_system, damage_digit_render_system,
    debug_render_collider_system, debug_render_polylines_setup_system,
    debug_render_polylines_update_system, debug_render_skeleton_system, effect_system,
    game_connection_system, game_mouse_input_system, game_state_enter_system,
    game_zone_change_system, hit_event_system, item_drop_model_add_collider_system,
    item_drop_model_system, login_connection_system, login_event_system, login_state_enter_system,
    login_state_exit_system, login_system, model_viewer_enter_system, model_viewer_exit_system,
    model_viewer_system, name_tag_system, name_tag_update_color_system,
    name_tag_update_healthbar_system, name_tag_visibility_system, network_thread_system,
    npc_idle_sound_system, npc_model_add_collider_system, npc_model_update_system,
    particle_sequence_system, passive_recovery_system, pending_damage_system,
    pending_skill_effect_system, personal_store_model_add_collider_system,
    personal_store_model_system, player_command_system, projectile_system, quest_trigger_system,
    spawn_effect_system, spawn_projectile_system, system_func_event_system, update_position_system,
    vehicle_model_system, visible_status_effects_system, world_connection_system,
    world_time_system, zone_time_system, zone_viewer_enter_system, DebugInspectorPlugin,
};
use ui::{
    load_dialog_sprites_system, ui_bank_system, ui_character_create_system,
    ui_character_info_system, ui_character_select_name_tag_system, ui_character_select_system,
    ui_chatbox_system, ui_debug_camera_info_system, ui_debug_client_entity_list_system,
    ui_debug_command_viewer_system, ui_debug_diagnostics_system, ui_debug_dialog_list_system,
    ui_debug_effect_list_system, ui_debug_entity_inspector_system, ui_debug_item_list_system,
    ui_debug_menu_system, ui_debug_npc_list_system, ui_debug_physics_system,
    ui_debug_render_system, ui_debug_skill_list_system, ui_debug_zone_lighting_system,
    ui_debug_zone_list_system, ui_debug_zone_time_system, ui_drag_and_drop_system,
    ui_game_menu_system, ui_hotbar_system, ui_inventory_system, ui_login_system,
    ui_message_box_system, ui_minimap_system, ui_npc_store_system, ui_number_input_dialog_system,
    ui_party_option_system, ui_party_system, ui_personal_store_system, ui_player_info_system,
    ui_quest_list_system, ui_selected_target_system, ui_server_select_system, ui_settings_system,
    ui_skill_list_system, ui_skill_tree_system, widgets::Dialog, DialogLoader, UiStateDebugWindows,
    UiStateDragAndDrop, UiStateWindows,
};
use vfs_asset_io::VfsAssetIo;
use zmo_asset_loader::{ZmoAsset, ZmoAssetLoader};
use zms_asset_loader::{ZmsAssetLoader, ZmsMaterialNumFaces};
use zone_loader::{zone_loader_system, ZoneLoader, ZoneLoaderAsset};

use crate::components::SoundCategory;

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct AccountConfig {
    pub username: String,
    pub password: String,
}

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct AutoLoginConfig {
    pub enabled: bool,
    pub channel_id: Option<usize>,
    pub server_id: Option<usize>,
    pub character_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(tag = "type", content = "path")]
pub enum FilesystemDeviceConfig {
    #[serde(rename = "vfs")]
    Vfs(String),
    #[serde(rename = "directory")]
    Directory(String),
    #[serde(rename = "aruavfs")]
    AruaVfs(String),
    #[serde(rename = "titanvfs")]
    TitanVfs(String),
}

#[derive(Deserialize)]
#[serde(default)]
pub struct FilesystemConfig {
    pub devices: Vec<FilesystemDeviceConfig>,
}

impl Default for FilesystemConfig {
    fn default() -> Self {
        let mut devices = Vec::new();

        if Path::new("data.idx").exists() {
            devices.push(FilesystemDeviceConfig::Vfs("data.idx".into()));
        }

        Self { devices }
    }
}

impl FilesystemConfig {
    pub fn create_virtual_filesystem(&self) -> Option<Arc<VirtualFilesystem>> {
        let mut vfs_devices: Vec<Box<dyn VirtualFilesystemDevice + Send + Sync>> = Vec::new();
        for device_config in self.devices.iter() {
            match device_config {
                FilesystemDeviceConfig::Directory(path) => {
                    log::info!("Loading game data from host directory {}", path);
                    vfs_devices.push(Box::new(HostFilesystemDevice::new(path.into())));
                }
                FilesystemDeviceConfig::AruaVfs(path) => {
                    let index_root_path = Path::new(path)
                        .parent()
                        .map(|path| path.into())
                        .unwrap_or_else(PathBuf::new);

                    log::info!("Loading game data from AruaVfs {}", path);
                    vfs_devices.push(Box::new(
                        AruaVfsIndex::load(Path::new(path), &index_root_path.join("data.rose"))
                            .unwrap_or_else(|_| panic!("Failed to load AruaVfs at {}", path)),
                    ));

                    log::info!(
                        "Loading game data from AruaVfs root path {}",
                        index_root_path.to_string_lossy()
                    );
                    vfs_devices.push(Box::new(HostFilesystemDevice::new(index_root_path)));
                }
                FilesystemDeviceConfig::TitanVfs(path) => {
                    let index_root_path = Path::new(path)
                        .parent()
                        .map(|path| path.into())
                        .unwrap_or_else(PathBuf::new);

                    log::info!("Loading game data from TitanVfs {}", path);
                    vfs_devices.push(Box::new(
                        TitanVfsIndex::load(Path::new(path), &index_root_path.join("data.trf"))
                            .unwrap_or_else(|_| panic!("Failed to load TitanVfs at {}", path)),
                    ));

                    log::info!("Loading game data from TitanVfs root path {}", path);
                    vfs_devices.push(Box::new(HostFilesystemDevice::new(index_root_path)));
                }
                FilesystemDeviceConfig::Vfs(path) => {
                    log::info!("Loading game data from Vfs {}", path);
                    vfs_devices.push(Box::new(
                        VfsIndex::load(Path::new(path))
                            .unwrap_or_else(|_| panic!("Failed to load Vfs at {}", path)),
                    ));

                    let index_root_path = Path::new(path)
                        .parent()
                        .map(|path| path.into())
                        .unwrap_or_else(PathBuf::new);
                    log::info!("Loading game data from Vfs root path {}", path);
                    vfs_devices.push(Box::new(HostFilesystemDevice::new(index_root_path)));
                }
            }
        }

        if vfs_devices.is_empty() {
            None
        } else {
            Some(Arc::new(VirtualFilesystem::new(vfs_devices)))
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub ip: String,
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            ip: "127.0.0.1".into(),
            port: 29000,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
pub struct GameConfig {
    pub data_version: String,
    pub network_version: String,
    pub ui_version: String,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            data_version: "irose".into(),
            network_version: "irose".into(),
            ui_version: "irose".into(),
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum GraphicsModeConfig {
    #[serde(rename = "window")]
    Window { width: f32, height: f32 },
    #[serde(rename = "fullscreen")]
    Fullscreen,
}

#[derive(Deserialize)]
#[serde(default)]
pub struct GraphicsConfig {
    pub mode: GraphicsModeConfig,
    pub passthrough_terrain_textures: bool,
    pub trail_effect_duration_multiplier: f32,
    pub disable_vsync: bool,
}

impl Default for GraphicsConfig {
    fn default() -> Self {
        Self {
            mode: GraphicsModeConfig::Window {
                width: 1920.0,
                height: 1080.0,
            },
            passthrough_terrain_textures: false,
            trail_effect_duration_multiplier: 1.0,
            disable_vsync: false,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
pub struct SoundVolumeConfig {
    pub global: f32,
    pub background_music: f32,
    pub player_footstep: f32,
    pub player_combat: f32,
    pub other_footstep: f32,
    pub other_combat: f32,
    pub npc_sounds: f32,
}

impl Default for SoundVolumeConfig {
    fn default() -> Self {
        Self {
            global: 0.6,
            background_music: 0.15,
            player_footstep: 0.9,
            player_combat: 1.0,
            other_footstep: 0.5,
            other_combat: 0.5,
            npc_sounds: 0.6,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
pub struct SoundConfig {
    pub enabled: bool,
    pub volume: SoundVolumeConfig,
}

impl Default for SoundConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            volume: SoundVolumeConfig::default(),
        }
    }
}

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub account: AccountConfig,
    pub auto_login: AutoLoginConfig,
    pub filesystem: FilesystemConfig,
    pub game: GameConfig,
    pub graphics: GraphicsConfig,
    pub server: ServerConfig,
    pub sound: SoundConfig,
}

pub fn load_config(path: &Path) -> Config {
    let toml_str = match std::fs::read_to_string(path) {
        Ok(toml_str) => toml_str,
        Err(error) => {
            println!(
                "Failed to load configuration from {} with error: {}",
                path.to_string_lossy(),
                error
            );
            return Config::default();
        }
    };

    match toml::from_str(&toml_str) {
        Ok(config) => {
            println!("Read configuration from {}", path.to_string_lossy());
            config
        }
        Err(error) => {
            println!(
                "Failed to load configuration from {} with error: {}",
                path.to_string_lossy(),
                error
            );
            Config::default()
        }
    }
}

#[derive(Default)]
pub struct SystemsConfig {
    pub disable_player_command_system: bool,
    pub add_custom_systems: Option<Box<dyn FnOnce(&mut App)>>,
}

pub fn run_game(config: &Config, systems_config: SystemsConfig) {
    run_client(config, AppState::GameLogin, systems_config);
}

pub fn run_model_viewer(config: &Config) {
    run_client(config, AppState::ModelViewer, SystemsConfig::default());
}

pub fn run_zone_viewer(config: &Config, zone_id: Option<ZoneId>) {
    run_client(
        config,
        AppState::ZoneViewer,
        SystemsConfig {
            add_custom_systems: Some(Box::new(move |app| {
                app.world
                    .resource_mut::<Events<LoadZoneEvent>>()
                    .send(LoadZoneEvent::new(
                        zone_id.unwrap_or_else(|| ZoneId::new(1).unwrap()),
                    ));
            })),
            ..Default::default()
        },
    );
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, StageLabel)]
enum GameStages {
    Network,
    ZoneChange,
    DebugRender,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, StageLabel)]
enum ModelViewerStages {
    Input,
}

fn run_client(config: &Config, app_state: AppState, mut systems_config: SystemsConfig) {
    let virtual_filesystem =
        if let Some(virtual_filesystem) = config.filesystem.create_virtual_filesystem() {
            virtual_filesystem
        } else {
            log::error!("No filesystem devices");
            return;
        };

    let (window_width, window_height) =
        if let GraphicsModeConfig::Window { width, height } = config.graphics.mode {
            (width, height)
        } else {
            (1920.0, 1080.0)
        };

    let mut app = App::new();

    // Must Initialise asset server before asset plugin
    app.insert_resource(VfsResource {
        vfs: virtual_filesystem.clone(),
    })
    .insert_resource(AssetServer::new(VfsAssetIo::new(virtual_filesystem)));

    // Initialise bevy engine
    app.insert_resource(Msaa { samples: 4 })
        .insert_resource(AssetServerSettings::default())
        .insert_resource(WindowDescriptor {
            title: "rose-offline-client".to_string(),
            present_mode: if config.graphics.disable_vsync {
                bevy::window::PresentMode::Immediate
            } else {
                bevy::window::PresentMode::Fifo
            },
            width: window_width,
            height: window_height,
            mode: if matches!(config.graphics.mode, GraphicsModeConfig::Fullscreen) {
                WindowMode::BorderlessFullscreen
            } else {
                WindowMode::Windowed
            },
            ..Default::default()
        })
        .insert_resource(ClearColor(Color::rgb(0.70, 0.90, 1.0)))
        .insert_resource(WgpuSettings {
            features: WgpuFeatures::TEXTURE_COMPRESSION_BC,
            ..Default::default()
        })
        .insert_resource(LogSettings {
            level: Level::INFO,
            filter: "wgpu=error,packets=debug,quest=trace,lua=debug,con=trace,animation=info"
                .to_string(),
        })
        .add_plugin(bevy::log::LogPlugin::default())
        .add_plugin(bevy::core::CorePlugin::default())
        .add_plugin(bevy::time::TimePlugin::default())
        .add_plugin(bevy::diagnostic::EntityCountDiagnosticsPlugin::default())
        .add_plugin(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default())
        .add_plugin(bevy::transform::TransformPlugin::default())
        .add_plugin(bevy::hierarchy::HierarchyPlugin::default())
        .add_plugin(bevy::diagnostic::DiagnosticsPlugin::default())
        .add_plugin(bevy::input::InputPlugin::default())
        .add_plugin(bevy::window::WindowPlugin::default())
        .add_plugin(bevy::asset::AssetPlugin::default())
        .add_plugin(bevy::scene::ScenePlugin::default())
        .add_plugin(bevy::winit::WinitPlugin::default())
        .add_plugin(bevy::render::RenderPlugin::default())
        .add_plugin(bevy::core_pipeline::CorePipelinePlugin::default())
        .add_plugin(bevy::pbr::PbrPlugin::default());

    // Initialise 3rd party bevy plugins
    app.add_plugin(bevy_polyline::PolylinePlugin)
        .add_plugin(bevy_egui::EguiPlugin)
        .add_plugin(FreeCameraPlugin)
        .add_plugin(OrbitCameraPlugin)
        .add_plugin(bevy_rapier3d::prelude::RapierPhysicsPlugin::<
            bevy_rapier3d::prelude::NoUserData,
        >::default())
        .insert_resource(bevy_rapier3d::prelude::RapierConfiguration {
            physics_pipeline_active: false,
            query_pipeline_active: true,
            ..Default::default()
        })
        .add_plugin(OddioPlugin);

    // Initialise rose stuff
    app.init_asset_loader::<ZmsAssetLoader>()
        .add_asset::<ZmoAsset>()
        .add_asset::<ZmsMaterialNumFaces>()
        .init_asset_loader::<ZmoAssetLoader>()
        .add_asset::<ZoneLoaderAsset>()
        .init_asset_loader::<DialogLoader>()
        .add_asset::<Dialog>()
        .insert_resource(RenderConfiguration {
            passthrough_terrain_textures: config.graphics.passthrough_terrain_textures,
            trail_effect_duration_multiplier: config.graphics.trail_effect_duration_multiplier,
        })
        .insert_resource(ServerConfiguration {
            ip: config.server.ip.clone(),
            port: format!("{}", config.server.port),
            preset_username: Some(config.account.username.clone()),
            preset_password: Some(config.account.password.clone()),
            preset_server_id: config.auto_login.server_id,
            preset_channel_id: config.auto_login.channel_id,
            preset_character_name: config.auto_login.character_name.clone(),
            auto_login: config.auto_login.enabled,
        })
        .insert_resource(SoundSettings {
            enabled: config.sound.enabled,
            global_gain: config.sound.volume.global,
            gains: enum_map! {
                SoundCategory::BackgroundMusic => config.sound.volume.background_music,
                SoundCategory::PlayerFootstep => config.sound.volume.player_footstep,
                SoundCategory::PlayerCombat => config.sound.volume.player_combat,
                SoundCategory::OtherFootstep => config.sound.volume.other_footstep,
                SoundCategory::OtherCombat => config.sound.volume.other_combat,
                SoundCategory::NpcSounds => config.sound.volume.npc_sounds,
            },
        })
        .add_plugin(RoseRenderPlugin)
        .add_plugin(RoseScriptingPlugin)
        .add_plugin(DebugInspectorPlugin);

    // Setup state
    app.add_state(app_state);

    app.init_resource::<Events<AnimationFrameEvent>>()
        .init_resource::<Events<BankEvent>>()
        .init_resource::<Events<ChatboxEvent>>()
        .init_resource::<Events<CharacterSelectEvent>>()
        .init_resource::<Events<ClientEntityEvent>>()
        .init_resource::<Events<ConversationDialogEvent>>()
        .init_resource::<Events<GameConnectionEvent>>()
        .init_resource::<Events<HitEvent>>()
        .init_resource::<Events<LoginEvent>>()
        .init_resource::<Events<LoadZoneEvent>>()
        .init_resource::<Events<MessageBoxEvent>>()
        .init_resource::<Events<NetworkEvent>>()
        .init_resource::<Events<NumberInputDialogEvent>>()
        .init_resource::<Events<NpcStoreEvent>>()
        .init_resource::<Events<PartyEvent>>()
        .init_resource::<Events<PersonalStoreEvent>>()
        .init_resource::<Events<PlayerCommandEvent>>()
        .init_resource::<Events<QuestTriggerEvent>>()
        .init_resource::<Events<SystemFuncEvent>>()
        .init_resource::<Events<SpawnEffectEvent>>()
        .init_resource::<Events<SpawnProjectileEvent>>()
        .init_resource::<Events<WorldConnectionEvent>>()
        .init_resource::<Events<ZoneEvent>>();

    app.add_system(auto_login_system)
        .add_system(background_music_system)
        .add_system(character_model_update_system)
        .add_system(character_model_add_collider_system.after(character_model_update_system))
        .add_system(personal_store_model_system)
        .add_system(personal_store_model_add_collider_system.after(personal_store_model_system))
        .add_system(npc_model_update_system)
        .add_system(npc_model_add_collider_system.after(npc_model_update_system))
        .add_system(item_drop_model_system)
        .add_system(item_drop_model_add_collider_system.after(item_drop_model_system))
        .add_system(vehicle_model_system.after(character_model_update_system))
        .add_system(
            animation_system
                .after(character_model_update_system)
                .after(npc_model_update_system),
        )
        .add_system(particle_sequence_system)
        .add_system(effect_system)
        .add_system(
            animation_effect_system
                .after(animation_system)
                .before(spawn_effect_system),
        )
        .add_system(animation_sound_system.after(animation_system))
        .add_system(
            projectile_system
                .after(animation_effect_system)
                .before(spawn_effect_system),
        )
        .add_system(visible_status_effects_system.before(spawn_effect_system))
        .add_system(
            spawn_projectile_system
                .after(animation_effect_system)
                .before(spawn_effect_system),
        )
        .add_system(
            pending_damage_system
                .after(animation_effect_system)
                .after(projectile_system),
        )
        .add_system(
            pending_skill_effect_system
                .after(animation_effect_system)
                .after(projectile_system),
        )
        .add_system(
            hit_event_system
                .after(animation_effect_system)
                .after(pending_skill_effect_system)
                .after(projectile_system),
        )
        .add_system(
            damage_digit_render_system
                .after(pending_damage_system)
                .after(hit_event_system),
        )
        .add_system(
            name_tag_update_healthbar_system
                .after(pending_damage_system)
                .after(hit_event_system),
        )
        .add_system(update_ui_resources)
        .add_system(spawn_effect_system)
        .add_system(npc_idle_sound_system)
        .add_system(name_tag_system)
        .add_system(name_tag_visibility_system.after(game_mouse_input_system))
        .add_system(name_tag_update_color_system)
        .add_system(world_time_system)
        .add_system(system_func_event_system)
        .add_system(load_dialog_sprites_system)
        .add_system(zone_time_system.after(world_time_system))
        .add_system(ui_message_box_system.after("ui_system"))
        .add_system(ui_number_input_dialog_system.after("ui_system"))
        .add_system(ui_debug_camera_info_system.label("ui_system"))
        .add_system(ui_debug_client_entity_list_system.label("ui_system"))
        .add_system(ui_debug_command_viewer_system.label("ui_system"))
        .add_system(ui_debug_dialog_list_system.label("ui_system"))
        .add_system(ui_debug_effect_list_system.label("ui_system"))
        .add_system(ui_debug_item_list_system.label("ui_system"))
        .add_system(ui_debug_menu_system.before("ui_system"))
        .add_system(ui_debug_npc_list_system.label("ui_system"))
        .add_system(ui_debug_physics_system.label("ui_system"))
        .add_system(ui_debug_render_system.label("ui_system"))
        .add_system(ui_debug_skill_list_system.label("ui_system"))
        .add_system(ui_debug_zone_lighting_system.label("ui_system"))
        .add_system(ui_debug_zone_list_system.label("ui_system"))
        .add_system(ui_debug_zone_time_system.label("ui_system"))
        .add_system(ui_debug_diagnostics_system.label("ui_system"))
        .add_system(
            ui_debug_entity_inspector_system
                .exclusive_system()
                .label("ui_system"),
        );

    // character_model_blink_system in PostUpdate to avoid any conflicts with model destruction
    // e.g. through the character select exit system.
    app.add_system_to_stage(CoreStage::PostUpdate, character_model_blink_system);

    // Run zone change system just before physics sync which is after Update
    app.add_stage_before(
        PhysicsStages::SyncBackend,
        GameStages::ZoneChange,
        SystemStage::parallel()
            .with_system(zone_loader_system)
            .with_system(game_zone_change_system.after(zone_loader_system)),
    );

    // Run debug render stage last after physics update so it has accurate data
    app.add_startup_system(debug_render_polylines_setup_system);
    app.add_stage_after(
        PhysicsStages::Writeback,
        GameStages::DebugRender,
        SystemStage::parallel()
            .with_system(debug_render_collider_system.before(debug_render_polylines_update_system))
            .with_system(debug_render_skeleton_system.before(debug_render_polylines_update_system))
            .with_system(debug_render_polylines_update_system),
    );

    // Zone Viewer
    app.add_system_set(
        SystemSet::on_enter(AppState::ZoneViewer).with_system(zone_viewer_enter_system),
    );

    // Model Viewer, we avoid deleting any entities during CoreStage::Update by using a custom
    // stage which runs after Update. We cannot run before Update because the on_enter system
    // below will have not run yet.
    app.add_system_set(
        SystemSet::on_enter(AppState::ModelViewer).with_system(model_viewer_enter_system),
    );
    app.add_system_set(
        SystemSet::on_exit(AppState::ModelViewer).with_system(model_viewer_exit_system),
    );
    app.add_stage_after(
        CoreStage::Update,
        ModelViewerStages::Input,
        SystemStage::parallel()
            .with_system(model_viewer_system)
            .with_run_criteria(|state: Res<State<AppState>>| -> ShouldRun {
                if matches!(state.current(), AppState::ModelViewer) {
                    ShouldRun::Yes
                } else {
                    ShouldRun::No
                }
            }),
    );

    // Game Login
    app.add_system_set(
        SystemSet::on_enter(AppState::GameLogin).with_system(login_state_enter_system),
    )
    .add_system_set(SystemSet::on_exit(AppState::GameLogin).with_system(login_state_exit_system))
    .add_system_set(
        SystemSet::on_update(AppState::GameLogin)
            .with_system(login_system)
            .with_system(
                ui_login_system
                    .label("ui_system")
                    .after(login_system)
                    .before(login_event_system),
            )
            .with_system(
                ui_server_select_system
                    .label("ui_system")
                    .after(login_system)
                    .before(login_event_system),
            )
            .with_system(login_event_system),
    );

    // Game Character Select
    app.add_system_set(
        SystemSet::on_enter(AppState::GameCharacterSelect)
            .with_system(character_select_enter_system),
    )
    .add_system_set(
        SystemSet::on_update(AppState::GameCharacterSelect)
            .with_system(character_select_system)
            .with_system(character_select_input_system)
            .with_system(character_select_models_system)
            .with_system(
                ui_character_create_system
                    .label("ui_system")
                    .after(character_select_system)
                    .after(character_select_input_system)
                    .before(character_select_event_system),
            )
            .with_system(
                ui_character_select_system
                    .label("ui_system")
                    .after(character_select_system)
                    .after(character_select_input_system)
                    .before(character_select_event_system),
            )
            .with_system(character_select_event_system)
            .with_system(
                ui_character_select_name_tag_system
                    .label("ui_system")
                    .after(character_select_event_system),
            ),
    )
    .add_system_set(
        SystemSet::on_exit(AppState::GameCharacterSelect).with_system(character_select_exit_system),
    );

    // Game
    app.init_resource::<UiStateDragAndDrop>()
        .init_resource::<UiStateWindows>()
        .init_resource::<UiStateDebugWindows>()
        .init_resource::<ClientEntityList>()
        .init_resource::<DebugRenderConfig>()
        .init_resource::<WorldTime>()
        .init_resource::<ZoneTime>()
        .init_resource::<SelectedTarget>()
        .init_resource::<NameTagSettings>();

    app.add_system_set(SystemSet::on_enter(AppState::Game).with_system(game_state_enter_system))
        .add_system_set(
            SystemSet::on_update(AppState::Game)
                .with_system(ability_values_system)
                .with_system(command_system.after(animation_system))
                .with_system(update_position_system)
                .with_system(
                    collision_player_system_join_zoin
                        .after(update_position_system)
                        .before(collision_player_system),
                )
                .with_system(collision_height_only_system.after(update_position_system))
                .with_system(collision_player_system.after(update_position_system))
                .with_system(client_entity_event_system)
                .with_system(passive_recovery_system)
                .with_system(quest_trigger_system)
                .with_system(cooldown_system.before("ui_system"))
                .with_system(
                    game_mouse_input_system
                        .after("ui_system")
                        .after(ui_message_box_system)
                        .after(ui_number_input_dialog_system),
                )
                .with_system(ui_bank_system.label("ui_system"))
                .with_system(ui_chatbox_system.label("ui_system"))
                .with_system(ui_character_info_system.label("ui_system"))
                .with_system(ui_inventory_system.label("ui_system"))
                .with_system(
                    ui_game_menu_system
                        .label("ui_system")
                        .after(ui_character_info_system),
                )
                .with_system(ui_hotbar_system.label("ui_system"))
                .with_system(ui_minimap_system.label("ui_system"))
                .with_system(ui_npc_store_system.label("ui_system"))
                .with_system(ui_party_system.label("ui_system"))
                .with_system(ui_party_option_system.label("ui_system"))
                .with_system(ui_personal_store_system.label("ui_system"))
                .with_system(ui_player_info_system.label("ui_system"))
                .with_system(ui_quest_list_system.label("ui_system"))
                .with_system(ui_selected_target_system.label("ui_system"))
                .with_system(ui_skill_list_system.label("ui_system"))
                .with_system(ui_skill_tree_system.label("ui_system"))
                .with_system(ui_settings_system.label("ui_system"))
                .with_system(conversation_dialog_system.label("ui_system")),
        );

    if !systems_config.disable_player_command_system {
        app.add_system_set(
            SystemSet::on_update(AppState::Game)
                .with_system(player_command_system)
                .after(cooldown_system)
                .after(game_mouse_input_system),
        );
    }

    app.add_system_to_stage(CoreStage::PostUpdate, ui_drag_and_drop_system);

    // Setup network
    let (network_thread_tx, network_thread_rx) =
        tokio::sync::mpsc::unbounded_channel::<NetworkThreadMessage>();
    let network_thread = std::thread::spawn(move || run_network_thread(network_thread_rx));
    app.insert_resource(NetworkThread::new(network_thread_tx.clone()));

    // Run network systems before Update, so we can add/remove entities
    app.add_stage_before(
        CoreStage::Update,
        GameStages::Network,
        SystemStage::parallel()
            .with_system(login_connection_system)
            .with_system(world_connection_system)
            .with_system(game_connection_system),
    );

    app.add_startup_system_to_stage(StartupStage::PostStartup, load_common_game_data);

    if let Some(app_builder) = systems_config.add_custom_systems.take() {
        app_builder(&mut app);
    }

    match config.game.network_version.as_str() {
        "irose" => {
            app.add_system_to_stage(CoreStage::PostUpdate, network_thread_system);
        }
        "custom" => {}
        unknown => panic!("Unknown game network version {}", unknown),
    };

    match config.game.ui_version.as_str() {
        "irose" => {
            app.add_startup_system(load_ui_resources);
        }
        "custom" => {}
        unknown => panic!("Unknown game ui version {}", unknown),
    };

    match config.game.data_version.as_str() {
        "irose" => {
            app.add_startup_system(load_game_data_irose);
        }
        "custom" => {}
        unknown => panic!("Unknown game data version {}", unknown),
    };

    app.run();

    network_thread_tx.send(NetworkThreadMessage::Exit).ok();
    network_thread.join().ok();
}

fn load_game_data_irose(
    mut commands: Commands,
    vfs_resource: Res<VfsResource>,
    asset_server: Res<AssetServer>,
) {
    let string_database = rose_data_irose::get_string_database(&vfs_resource.vfs, 1)
        .expect("Failed to load string database");

    let items = Arc::new(
        rose_data_irose::get_item_database(&vfs_resource.vfs, string_database.clone())
            .expect("Failed to load item database"),
    );
    let npcs = Arc::new(
        rose_data_irose::get_npc_database(
            &vfs_resource.vfs,
            string_database.clone(),
            &NpcDatabaseOptions {
                load_frame_data: false,
            },
        )
        .expect("Failed to load npc database"),
    );
    let skills = Arc::new(
        rose_data_irose::get_skill_database(&vfs_resource.vfs, string_database.clone())
            .expect("Failed to load skill database"),
    );
    let character_motion_database = Arc::new(
        rose_data_irose::get_character_motion_database(
            &vfs_resource.vfs,
            &CharacterMotionDatabaseOptions {
                load_frame_data: false,
            },
        )
        .expect("Failed to load character motion list"),
    );
    let zone_list = Arc::new(
        rose_data_irose::get_zone_list(&vfs_resource.vfs, string_database.clone())
            .expect("Failed to load zone list"),
    );

    asset_server.add_loader(ZoneLoader {
        zone_list: zone_list.clone(),
    });

    commands.insert_resource(GameData {
        ability_value_calculator: rose_game_irose::data::get_ability_value_calculator(
            items.clone(),
            skills.clone(),
            npcs.clone(),
        ),
        animation_event_flags: rose_data_irose::get_animation_event_flags(),
        character_motion_database,
        client_strings: rose_data_irose::get_client_strings(string_database.clone())
            .expect("Failed to load client strings"),
        data_decoder: rose_data_irose::get_data_decoder(),
        effect_database: rose_data_irose::get_effect_database(&vfs_resource.vfs)
            .expect("Failed to load effect database"),
        items,
        job_class: Arc::new(
            rose_data_irose::get_job_class_database(&vfs_resource.vfs, string_database.clone())
                .expect("Failed to load job class database"),
        ),
        npcs,
        quests: Arc::new(
            rose_data_irose::get_quest_database(&vfs_resource.vfs, string_database.clone())
                .expect("Failed to load quest database"),
        ),
        skills,
        skybox: rose_data_irose::get_skybox_database(&vfs_resource.vfs)
            .expect("Failed to load skybox database"),
        sounds: rose_data_irose::get_sound_database(&vfs_resource.vfs)
            .expect("Failed to load sound database"),
        status_effects: Arc::new(
            rose_data_irose::get_status_effect_database(&vfs_resource.vfs, string_database.clone())
                .expect("Failed to load status effect database"),
        ),
        string_database,
        zone_list,
        ltb_event: vfs_resource
            .vfs
            .read_file::<LtbFile, _>("3DDATA/EVENT/ULNGTB_CON.LTB")
            .expect("Failed to load event language file"),
        zsc_event_object: vfs_resource
            .vfs
            .read_file::<ZscFile, _>("3DDATA/SPECIAL/EVENT_OBJECT.ZSC")
            .expect("Failed to load 3DDATA/SPECIAL/EVENT_OBJECT.ZSC"),
        zsc_special_object: vfs_resource
            .vfs
            .read_file::<ZscFile, _>("3DDATA/SPECIAL/LIST_DECO_SPECIAL.ZSC")
            .expect("Failed to load 3DDATA/SPECIAL/LIST_DECO_SPECIAL.ZSC"),
        stb_morph_object: vfs_resource
            .vfs
            .read_file::<StbFile, _>("3DDATA/STB/LIST_MORPH_OBJECT.STB")
            .expect("Failed to load 3DDATA/STB/LIST_MORPH_OBJECT.STB"),
        character_select_positions: vec![
            Transform::from_translation(Vec3::new(5205.0, 1.0, -5205.0))
                .with_rotation(Quat::from_xyzw(0.0, 1.0, 0.0, 0.0))
                .with_scale(Vec3::new(1.5, 1.5, 1.5)),
            Transform::from_translation(Vec3::new(5202.70, 1.0, -5206.53))
                .with_rotation(Quat::from_xyzw(0.0, 1.0, 0.0, 0.0))
                .with_scale(Vec3::new(1.5, 1.5, 1.5)),
            Transform::from_translation(Vec3::new(5200.00, 1.0, -5207.07))
                .with_rotation(Quat::from_xyzw(0.0, 1.0, 0.0, 0.0))
                .with_scale(Vec3::new(1.5, 1.5, 1.5)),
            Transform::from_translation(Vec3::new(5197.30, 1.0, -5206.53))
                .with_rotation(Quat::from_xyzw(0.0, 1.0, 0.0, 0.0))
                .with_scale(Vec3::new(1.5, 1.5, 1.5)),
            Transform::from_translation(Vec3::new(5195.00, 1.0, -5205.00))
                .with_rotation(Quat::from_xyzw(0.0, 1.0, 0.0, 0.0))
                .with_scale(Vec3::new(1.5, 1.5, 1.5)),
        ],
    });
}

fn load_common_game_data(
    mut commands: Commands,
    vfs_resource: Res<VfsResource>,
    game_data: Res<GameData>,
    asset_server: Res<AssetServer>,
    mut damage_digit_materials: ResMut<Assets<DamageDigitMaterial>>,
    mut egui_context: ResMut<EguiContext>,
) {
    commands.insert_resource(
        ModelLoader::new(
            vfs_resource.vfs.clone(),
            game_data.character_motion_database.clone(),
            game_data.effect_database.clone(),
            game_data.items.clone(),
            game_data.npcs.clone(),
            asset_server.load("3DDATA/EFFECT/TRAIL.DDS"),
        )
        .expect("Failed to create model loader"),
    );

    commands.spawn_bundle(Camera3dBundle::default());
    commands.insert_resource(AmbientLight {
        color: Color::rgb(1.0, 1.0, 1.0),
        brightness: 0.9,
    });

    commands.insert_resource(DamageDigitsSpawner::load(
        &asset_server,
        &mut damage_digit_materials,
    ));

    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "Ubuntu-M".to_owned(),
        egui::FontData::from_static(include_bytes!("fonts/Ubuntu-M.ttf")),
    );

    fonts
        .families
        .entry(egui::FontFamily::Name("Ubuntu-M".into()))
        .or_default()
        .insert(0, "Ubuntu-M".to_owned());

    egui_context.ctx_mut().set_fonts(fonts);
}
