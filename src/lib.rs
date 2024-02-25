#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

use animation::RoseAnimationPlugin;
use bevy::{
    core_pipeline::{bloom::BloomSettings, clear_color::ClearColor},
    ecs::event::Events,
    log::Level,
    prelude::{
        apply_deferred, in_state, AddAsset, App, AssetServer, Assets, Camera, Camera3dBundle,
        Color, Commands, IntoSystemConfigs, IntoSystemSetConfigs, Msaa, OnEnter, OnExit,
        PluginGroup, PostStartup, PostUpdate, PreUpdate, Quat, Res, ResMut, Startup, State,
        SystemSet, Transform, Update, Vec3,
    },
    render::{render_resource::WgpuFeatures, settings::WgpuSettings},
    transform::TransformSystem,
    window::{Window, WindowMode},
};
use bevy_egui::{egui, EguiContexts, EguiSet};
use bevy_rapier3d::plugin::PhysicsSet;
use enum_map::enum_map;
use exe_resource_loader::{ExeResourceCursor, ExeResourceLoader};
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use rose_data::{CharacterMotionDatabaseOptions, NpcDatabaseOptions, ZoneId};
use rose_file_readers::{
    AruaVfsIndex, HostFilesystemDevice, IrosePhVfsIndex, LtbFile, StbFile, TitanVfsIndex, VfsIndex,
    VirtualFilesystem, VirtualFilesystemDevice, ZscFile,
};

pub mod animation;
pub mod audio;
pub mod bundles;
pub mod components;
pub mod effect_loader;
pub mod events;
pub mod exe_resource_loader;
pub mod model_loader;
pub mod protocol;
pub mod render;
pub mod resources;
pub mod scripting;
pub mod systems;
pub mod ui;
pub mod vfs_asset_io;
pub mod zms_asset_loader;
pub mod zone_loader;

use audio::OddioPlugin;
use events::{
    BankEvent, CharacterSelectEvent, ChatboxEvent, ClanDialogEvent, ClientEntityEvent,
    ConversationDialogEvent, GameConnectionEvent, HitEvent, LoadZoneEvent, LoginEvent,
    MessageBoxEvent, MoveDestinationEffectEvent, NetworkEvent, NpcStoreEvent,
    NumberInputDialogEvent, PartyEvent, PersonalStoreEvent, PlayerCommandEvent, QuestTriggerEvent,
    SpawnEffectEvent, SpawnProjectileEvent, SystemFuncEvent, UseItemEvent, WorldConnectionEvent,
    ZoneEvent,
};
use model_loader::ModelLoader;
use render::{DamageDigitMaterial, RoseRenderPlugin};
use resources::{
    load_ui_resources, run_network_thread, ui_requested_cursor_apply_system, update_ui_resources,
    AppState, ClientEntityList, DamageDigitsSpawner, DebugRenderConfig, GameData, NameTagSettings,
    NetworkThread, NetworkThreadMessage, RenderConfiguration, SelectedTarget, ServerConfiguration,
    SoundCache, SoundSettings, SpecularTexture, VfsResource, WorldTime, ZoneTime,
};
use scripting::RoseScriptingPlugin;
use systems::{
    ability_values_system, animation_effect_system, animation_sound_system, auto_login_system,
    background_music_system, character_model_add_collider_system, character_model_blink_system,
    character_model_update_system, character_select_enter_system, character_select_event_system,
    character_select_exit_system, character_select_input_system, character_select_models_system,
    character_select_system, clan_system, client_entity_event_system, collision_height_only_system,
    collision_player_system, collision_player_system_join_zoin, command_system,
    conversation_dialog_system, cooldown_system, damage_digit_render_system,
    debug_render_collider_system, debug_render_directional_light_system,
    debug_render_skeleton_system, directional_light_system, effect_system, facing_direction_system,
    free_camera_system, game_connection_system, game_mouse_input_system, game_state_enter_system,
    game_zone_change_system, hit_event_system, item_drop_model_add_collider_system,
    item_drop_model_system, login_connection_system, login_event_system, login_state_enter_system,
    login_state_exit_system, login_system, model_viewer_enter_system, model_viewer_exit_system,
    model_viewer_system, move_destination_effect_system, name_tag_system,
    name_tag_update_color_system, name_tag_update_healthbar_system, name_tag_visibility_system,
    network_thread_system, npc_idle_sound_system, npc_model_add_collider_system,
    npc_model_update_system, orbit_camera_system, particle_sequence_system,
    passive_recovery_system, pending_damage_system, pending_skill_effect_system,
    personal_store_model_add_collider_system, personal_store_model_system, player_command_system,
    projectile_system, quest_trigger_system, spawn_effect_system, spawn_projectile_system,
    status_effect_system, system_func_event_system, update_position_system, use_item_event_system,
    vehicle_model_system, vehicle_sound_system, visible_status_effects_system,
    world_connection_system, world_time_system, zone_time_system, zone_viewer_enter_system,
    DebugInspectorPlugin,
};
use ui::{
    load_dialog_sprites_system, ui_bank_system, ui_character_create_system,
    ui_character_info_system, ui_character_select_name_tag_system, ui_character_select_system,
    ui_chatbox_system, ui_clan_system, ui_create_clan_system, ui_debug_camera_info_system,
    ui_debug_client_entity_list_system, ui_debug_command_viewer_system,
    ui_debug_diagnostics_system, ui_debug_dialog_list_system, ui_debug_effect_list_system,
    ui_debug_entity_inspector_system, ui_debug_item_list_system, ui_debug_menu_system,
    ui_debug_npc_list_system, ui_debug_physics_system, ui_debug_render_system,
    ui_debug_skill_list_system, ui_debug_zone_lighting_system, ui_debug_zone_list_system,
    ui_debug_zone_time_system, ui_drag_and_drop_system, ui_game_menu_system, ui_hotbar_system,
    ui_inventory_system, ui_item_drop_name_system, ui_login_system, ui_message_box_system,
    ui_minimap_system, ui_npc_store_system, ui_number_input_dialog_system, ui_party_option_system,
    ui_party_system, ui_personal_store_system, ui_player_info_system, ui_quest_list_system,
    ui_respawn_system, ui_selected_target_system, ui_server_select_system, ui_settings_system,
    ui_skill_list_system, ui_skill_tree_system, ui_sound_event_system, ui_status_effects_system,
    ui_window_sound_system, widgets::Dialog, DialogLoader, UiSoundEvent, UiStateDebugWindows,
    UiStateDragAndDrop, UiStateWindows,
};
use vfs_asset_io::VfsAssetIo;
use zms_asset_loader::{ZmsAssetLoader, ZmsMaterialNumFaces, ZmsNoSkinAssetLoader};
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
    #[serde(rename = "iroseph")]
    IrosePh(String),
}

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct FilesystemConfig {
    pub devices: Vec<FilesystemDeviceConfig>,
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
                FilesystemDeviceConfig::IrosePh(path) => {
                    let index_root_path = Path::new(path)
                        .parent()
                        .map(|path| path.into())
                        .unwrap_or_else(PathBuf::new);

                    log::info!("Loading game data from iRosePH {}", path);
                    vfs_devices.push(Box::new(
                        IrosePhVfsIndex::load(Path::new(path))
                            .unwrap_or_else(|_| panic!("Failed to load iRosePH VFS at {}", path)),
                    ));

                    log::info!(
                        "Loading game data from iRosePH root path {}",
                        index_root_path.to_string_lossy()
                    );
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
    pub ui_sounds: f32,
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
            ui_sounds: 0.5,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
enum GameStages {
    ZoneChange,
    ZoneChangeFlush,
    AfterUpdate,
    DebugRenderPreFlush,
    DebugRender,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum GameSystemSets {
    UpdateCamera,
    Ui,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum UiSystemSets {
    UiDebugMenu,
    UiFirst,
    Ui,
    UiLast,
    UiDebug,
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
    app.insert_resource(Msaa::Off)
        .insert_resource(ClearColor(Color::rgb(0.70, 0.90, 1.0)))
        .insert_resource(bevy::gizmos::GizmoConfig {
            depth_bias: -0.1,
            ..Default::default()
        })
        .add_plugins((
            bevy::prelude::DefaultPlugins
                .set(bevy::render::RenderPlugin {
                    wgpu_settings: WgpuSettings {
                        features: WgpuFeatures::TEXTURE_COMPRESSION_BC,
                        // backends: Some(Backends::DX12),
                        ..Default::default()
                    },
                })
                .set(bevy::window::WindowPlugin {
                    primary_window: Some(Window {
                        title: "rose-offline-client".to_string(),
                        present_mode: if config.graphics.disable_vsync {
                            bevy::window::PresentMode::Immediate
                        } else {
                            bevy::window::PresentMode::Fifo
                        },
                        resolution: bevy::window::WindowResolution::new(
                            window_width,
                            window_height,
                        ),
                        mode: if matches!(config.graphics.mode, GraphicsModeConfig::Fullscreen) {
                            WindowMode::BorderlessFullscreen
                        } else {
                            WindowMode::Windowed
                        },
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .set(bevy::log::LogPlugin {
                    level: Level::INFO,
                    filter:
                        "wgpu=error,packets=debug,quest=trace,lua=debug,con=trace,animation=info"
                            .to_string(),
                })
                .set(bevy::pbr::PbrPlugin {
                    prepass_enabled: false,
                }),
            bevy::diagnostic::EntityCountDiagnosticsPlugin,
            bevy::diagnostic::FrameTimeDiagnosticsPlugin,
        ));

    // Initialise 3rd party bevy plugins
    app.insert_resource(bevy_rapier3d::prelude::RapierConfiguration {
        physics_pipeline_active: false,
        query_pipeline_active: true,
        ..Default::default()
    });
    app.add_plugins((
        bevy_egui::EguiPlugin,
        bevy_rapier3d::prelude::RapierPhysicsPlugin::<bevy_rapier3d::prelude::NoUserData>::default(
        ),
        bevy_rapier3d::prelude::RapierDebugRenderPlugin {
            enabled: false,
            ..Default::default()
        },
        OddioPlugin,
    ));

    // Initialise rose stuff
    app.init_asset_loader::<ZmsAssetLoader>()
        .init_asset_loader::<ZmsNoSkinAssetLoader>()
        .add_asset::<ZmsMaterialNumFaces>()
        .add_asset::<ZoneLoaderAsset>()
        .init_asset_loader::<ExeResourceLoader>()
        .add_asset::<ExeResourceCursor>()
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
                SoundCategory::Ui => config.sound.volume.ui_sounds,
            },
        })
        .add_plugins((
            RoseAnimationPlugin,
            RoseRenderPlugin,
            RoseScriptingPlugin,
            DebugInspectorPlugin,
        ));

    // Setup state
    app.add_state::<AppState>()
        .insert_resource(State::new(app_state));

    app.add_event::<BankEvent>()
        .add_event::<ChatboxEvent>()
        .add_event::<CharacterSelectEvent>()
        .add_event::<ClanDialogEvent>()
        .add_event::<ClientEntityEvent>()
        .add_event::<ConversationDialogEvent>()
        .add_event::<GameConnectionEvent>()
        .add_event::<HitEvent>()
        .add_event::<LoginEvent>()
        .add_event::<LoadZoneEvent>()
        .add_event::<MessageBoxEvent>()
        .add_event::<MoveDestinationEffectEvent>()
        .add_event::<NetworkEvent>()
        .add_event::<NumberInputDialogEvent>()
        .add_event::<NpcStoreEvent>()
        .add_event::<PartyEvent>()
        .add_event::<PersonalStoreEvent>()
        .add_event::<PlayerCommandEvent>()
        .add_event::<QuestTriggerEvent>()
        .add_event::<SystemFuncEvent>()
        .add_event::<SpawnEffectEvent>()
        .add_event::<SpawnProjectileEvent>()
        .add_event::<UseItemEvent>()
        .add_event::<WorldConnectionEvent>()
        .add_event::<ZoneEvent>()
        .add_event::<UiSoundEvent>();

    app.add_systems(
        PostUpdate,
        (apply_deferred,).in_set(GameStages::ZoneChangeFlush),
    );
    app.add_systems(
        PostUpdate,
        (apply_deferred,).in_set(GameStages::DebugRenderPreFlush),
    );

    app.add_systems(
        Update,
        (free_camera_system, orbit_camera_system).in_set(GameSystemSets::UpdateCamera),
    );
    app.add_systems(
        Update,
        (
            (
                auto_login_system,
                background_music_system,
                character_model_update_system,
                character_model_add_collider_system.after(character_model_update_system),
                personal_store_model_system,
                personal_store_model_add_collider_system.after(personal_store_model_system),
                npc_model_update_system,
                npc_model_add_collider_system.after(npc_model_update_system),
                item_drop_model_system,
                item_drop_model_add_collider_system.after(item_drop_model_system),
                particle_sequence_system,
                effect_system,
                animation_effect_system.before(spawn_effect_system),
                animation_sound_system,
            ),
            (
                projectile_system
                    .after(animation_effect_system)
                    .before(spawn_effect_system),
                visible_status_effects_system.before(spawn_effect_system),
                spawn_projectile_system
                    .after(animation_effect_system)
                    .before(spawn_effect_system),
                pending_damage_system
                    .after(animation_effect_system)
                    .after(projectile_system),
                pending_skill_effect_system
                    .after(animation_effect_system)
                    .after(projectile_system),
                hit_event_system
                    .after(animation_effect_system)
                    .after(pending_skill_effect_system)
                    .after(projectile_system)
                    .before(spawn_effect_system),
                damage_digit_render_system
                    .after(pending_damage_system)
                    .after(hit_event_system),
                name_tag_update_healthbar_system
                    .after(pending_damage_system)
                    .after(hit_event_system),
            ),
            (
                update_ui_resources,
                spawn_effect_system,
                move_destination_effect_system.after(game_mouse_input_system),
                npc_idle_sound_system,
                name_tag_system,
                name_tag_visibility_system.after(game_mouse_input_system),
                name_tag_update_color_system,
                world_time_system,
                system_func_event_system,
                load_dialog_sprites_system,
                zone_time_system.after(world_time_system),
                directional_light_system,
            ),
        ),
    );

    app.add_systems(
        PostUpdate,
        ui_requested_cursor_apply_system.after(EguiSet::ProcessOutput),
    );

    app.add_systems(
        Update,
        ui_item_drop_name_system.in_set(UiSystemSets::UiFirst),
    );

    app.add_systems(
        Update,
        (ui_message_box_system, ui_number_input_dialog_system).in_set(UiSystemSets::UiLast),
    );
    app.add_systems(
        Update,
        (
            ui_window_sound_system.before(ui_sound_event_system),
            ui_sound_event_system,
        )
            .after(UiSystemSets::UiLast),
    );
    app.add_systems(
        Update,
        (ui_debug_menu_system,).in_set(UiSystemSets::UiDebugMenu),
    );

    app.add_systems(
        Update,
        (
            ui_debug_camera_info_system,
            ui_debug_client_entity_list_system,
            ui_debug_command_viewer_system,
            ui_debug_dialog_list_system,
            ui_debug_effect_list_system,
            ui_debug_entity_inspector_system,
            ui_debug_item_list_system,
            ui_debug_npc_list_system,
            ui_debug_physics_system,
            ui_debug_render_system,
            ui_debug_skill_list_system,
            ui_debug_zone_lighting_system,
            ui_debug_zone_list_system,
            ui_debug_zone_time_system,
            ui_debug_diagnostics_system,
        )
            .in_set(UiSystemSets::UiDebug),
    );

    // character_model_blink_system in PostUpdate to avoid any conflicts with model destruction
    // e.g. through the character select exit system.
    app.add_systems(PostUpdate, character_model_blink_system);

    // vehicle_model_system in after ::Update but before ::PostUpdate to avoid any conflicts,
    // with model destruction but to also be before global transform is calculated.
    app.add_systems(
        PostUpdate,
        (vehicle_model_system, vehicle_sound_system)
            .chain()
            .in_set(GameStages::AfterUpdate),
    );

    // Run zone change system just before physics sync which is after Update
    app.add_systems(
        Update,
        (
            zone_loader_system,
            game_zone_change_system.after(zone_loader_system),
        )
            .in_set(GameStages::ZoneChange),
    );

    // Run debug render stage last after physics update so it has accurate data
    app.add_systems(
        Update,
        (
            debug_render_collider_system,
            debug_render_skeleton_system,
            debug_render_directional_light_system,
        )
            .in_set(GameStages::DebugRender),
    );

    // Zone Viewer
    app.add_systems(OnEnter(AppState::ZoneViewer), zone_viewer_enter_system);

    // Model Viewer, we avoid deleting any entities during CoreStage::Update by using a custom
    // stage which runs after Update. We cannot run before Update because the on_enter system
    // below will have not run yet.
    app.add_systems(OnEnter(AppState::ModelViewer), model_viewer_enter_system);
    app.add_systems(OnExit(AppState::ModelViewer), model_viewer_exit_system);
    app.add_systems(
        PostUpdate,
        model_viewer_system
            .run_if(in_state(AppState::ModelViewer))
            .in_set(GameStages::ZoneChange)
            .before(EguiSet::ProcessOutput), // model_viewer_system renders UI so must be before egui
    );

    // Game Login
    app.add_systems(OnEnter(AppState::GameLogin), login_state_enter_system)
        .add_systems(OnExit(AppState::GameLogin), login_state_exit_system);

    app.add_systems(
        Update,
        (login_system, login_event_system).run_if(in_state(AppState::GameLogin)),
    );

    app.add_systems(
        Update,
        (ui_login_system, ui_server_select_system)
            .run_if(in_state(AppState::GameLogin))
            .in_set(UiSystemSets::Ui)
            .after(login_system)
            .before(login_event_system),
    );

    // Game Character Select
    app.add_systems(
        OnEnter(AppState::GameCharacterSelect),
        character_select_enter_system,
    )
    .add_systems(
        OnExit(AppState::GameCharacterSelect),
        character_select_exit_system,
    );

    app.add_systems(
        Update,
        (
            character_select_system,
            character_select_input_system,
            character_select_models_system,
            character_select_event_system,
        )
            .run_if(in_state(AppState::GameCharacterSelect)),
    );

    app.add_systems(
        Update,
        (
            ui_character_create_system,
            ui_character_select_system,
            ui_character_select_name_tag_system,
        )
            .run_if(in_state(AppState::GameCharacterSelect))
            .in_set(UiSystemSets::Ui)
            .after(character_select_system)
            .after(character_select_input_system)
            .before(character_select_event_system),
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

    app.add_systems(OnEnter(AppState::Game), game_state_enter_system);

    app.add_systems(
        Update,
        (
            ability_values_system,
            clan_system,
            command_system
                .after(npc_model_update_system)
                .after(npc_model_add_collider_system)
                .after(spawn_effect_system),
            facing_direction_system.after(command_system),
            update_position_system.before(directional_light_system),
            collision_player_system_join_zoin
                .after(update_position_system)
                .before(collision_player_system),
            collision_height_only_system.after(update_position_system),
            collision_player_system.after(update_position_system),
            cooldown_system.before(GameSystemSets::Ui),
            client_entity_event_system.before(spawn_effect_system),
            use_item_event_system.before(spawn_effect_system),
            status_effect_system,
            passive_recovery_system,
            quest_trigger_system,
            game_mouse_input_system.after(GameSystemSets::Ui),
        )
            .run_if(in_state(AppState::Game)),
    );

    app.add_systems(
        Update,
        (
            (
                ui_bank_system,
                ui_chatbox_system,
                ui_character_info_system,
                ui_clan_system,
                ui_create_clan_system,
                ui_inventory_system,
                ui_game_menu_system.after(ui_character_info_system),
                ui_hotbar_system,
                ui_minimap_system,
                ui_npc_store_system,
                ui_party_system,
                ui_party_option_system,
                ui_personal_store_system,
                ui_player_info_system,
            ),
            (
                ui_quest_list_system,
                ui_respawn_system,
                ui_selected_target_system,
                ui_skill_list_system,
                ui_skill_tree_system,
                ui_settings_system,
                ui_status_effects_system,
                conversation_dialog_system,
            ),
        )
            .run_if(in_state(AppState::Game))
            .in_set(UiSystemSets::Ui),
    );

    if !systems_config.disable_player_command_system {
        app.add_systems(
            Update,
            (player_command_system
                .after(cooldown_system)
                .after(game_mouse_input_system),)
                .run_if(in_state(AppState::Game)),
        );
    }

    app.add_systems(PostUpdate, ui_drag_and_drop_system);

    // Setup network
    let (network_thread_tx, network_thread_rx) =
        tokio::sync::mpsc::unbounded_channel::<NetworkThreadMessage>();
    let network_thread = std::thread::spawn(move || run_network_thread(network_thread_rx));
    app.insert_resource(NetworkThread::new(network_thread_tx.clone()));

    // Run network systems before Update, so we can add/remove entities
    app.add_systems(
        PreUpdate,
        (
            login_connection_system,
            world_connection_system,
            game_connection_system,
        ),
    );

    app.add_systems(PostStartup, load_common_game_data);

    if let Some(app_builder) = systems_config.add_custom_systems.take() {
        app_builder(&mut app);
    }

    match config.game.network_version.as_str() {
        "irose" => {
            app.add_systems(PostUpdate, network_thread_system);
        }
        "custom" => {}
        unknown => panic!("Unknown game network version {}", unknown),
    };

    match config.game.ui_version.as_str() {
        "irose" => {
            app.add_systems(Startup, load_ui_resources);
        }
        "custom" => {}
        unknown => panic!("Unknown game ui version {}", unknown),
    };

    match config.game.data_version.as_str() {
        "irose" => {
            app.add_systems(Startup, load_game_data_irose);
        }
        "custom" => {}
        unknown => panic!("Unknown game data version {}", unknown),
    };

    app.configure_sets(
        PostUpdate,
        (GameStages::AfterUpdate,).before(PhysicsSet::SyncBackend),
    );

    app.configure_sets(
        PostUpdate,
        (
            GameStages::ZoneChange,
            GameStages::ZoneChangeFlush,
            GameStages::AfterUpdate,
        )
            .chain()
            .before(PhysicsSet::SyncBackend),
    );

    app.configure_sets(
        PostUpdate,
        (GameStages::DebugRenderPreFlush, GameStages::DebugRender)
            .chain()
            .after(TransformSystem::TransformPropagate),
    );

    app.configure_sets(
        Update,
        (
            UiSystemSets::UiDebugMenu,
            UiSystemSets::UiFirst,
            UiSystemSets::Ui,
            UiSystemSets::UiLast,
            UiSystemSets::UiDebug,
        )
            .chain()
            .in_set(GameSystemSets::Ui),
    );

    app.configure_sets(
        Update,
        (GameSystemSets::UpdateCamera, GameSystemSets::Ui).chain(),
    );

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
    let sounds = rose_data_irose::get_sound_database(&vfs_resource.vfs)
        .expect("Failed to load sound database");

    asset_server.add_loader(ZoneLoader {
        zone_list: zone_list.clone(),
    });

    commands.insert_resource(SoundCache::new(sounds.len()));

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
        sounds,
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
    mut egui_context: EguiContexts,
) {
    commands.insert_resource(SpecularTexture {
        image: asset_server.load("ETC/SPECULAR_SPHEREMAP.DDS"),
    });

    commands.insert_resource(
        ModelLoader::new(
            vfs_resource.vfs.clone(),
            game_data.character_motion_database.clone(),
            game_data.effect_database.clone(),
            game_data.items.clone(),
            game_data.npcs.clone(),
            asset_server.load("3DDATA/EFFECT/TRAIL.DDS"),
            asset_server.load("ETC/SPECULAR_SPHEREMAP.DDS"),
        )
        .expect("Failed to create model loader"),
    );

    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                hdr: false,
                ..Default::default()
            },
            ..Default::default()
        },
        BloomSettings::NATURAL,
    ));

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
