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
        ParallelSystemDescriptorCoercion, Res, ResMut, StageLabel, State, SystemSet, SystemStage,
    },
    render::{render_resource::WgpuFeatures, settings::WgpuSettings},
    window::WindowDescriptor,
};
use bevy_egui::EguiContext;
use bevy_rapier3d::plugin::PhysicsStages;
use enum_map::enum_map;
use std::{path::Path, sync::Arc};

use rose_data::{CharacterMotionDatabaseOptions, NpcDatabaseOptions, ZoneId};
use rose_file_readers::{LtbFile, StbFile, StlFile, StlReadOptions, VfsIndex, ZscFile};

mod audio;
mod bundles;
mod components;
mod effect_loader;
mod events;
mod free_camera;
mod model_loader;
mod orbit_camera;
mod protocol;
mod ray_from_screenspace;
mod render;
mod resources;
mod scripting;
mod systems;
mod ui;
mod vfs_asset_io;
mod zmo_asset_loader;
mod zms_asset_loader;
mod zone_loader;

use audio::OddioPlugin;
use events::{
    AnimationFrameEvent, ChatboxEvent, ClientEntityEvent, ConversationDialogEvent,
    GameConnectionEvent, HitEvent, LoadZoneEvent, NpcStoreEvent, PartyEvent, PlayerCommandEvent,
    QuestTriggerEvent, SpawnEffectEvent, SpawnProjectileEvent, SystemFuncEvent,
    WorldConnectionEvent, ZoneEvent,
};
use free_camera::FreeCameraPlugin;
use model_loader::ModelLoader;
use orbit_camera::OrbitCameraPlugin;
use protocol::ProtocolType;
use render::{DamageDigitMaterial, RoseRenderPlugin};
use resources::{
    load_ui_resources, run_network_thread, update_ui_resources, AppState, ClientEntityList,
    DamageDigitsSpawner, DebugRenderConfig, GameData, Icons, NetworkThread, NetworkThreadMessage,
    RenderConfiguration, ServerConfiguration, SoundSettings, WorldTime, ZoneTime,
};
use scripting::RoseScriptingPlugin;
use systems::{
    ability_values_system, animation_effect_system, animation_sound_system, animation_system,
    background_music_system, character_model_add_collider_system, character_model_blink_system,
    character_model_system, character_select_enter_system, character_select_exit_system,
    character_select_input_system, character_select_models_system, character_select_system,
    client_entity_event_system, collision_height_only_system, collision_player_system,
    collision_player_system_join_zoin, command_system, conversation_dialog_system, cooldown_system,
    damage_digit_render_system, debug_render_collider_system, debug_render_polylines_setup_system,
    debug_render_polylines_update_system, debug_render_skeleton_system, effect_system,
    game_connection_system, game_mouse_input_system, game_state_enter_system,
    game_zone_change_system, hit_event_system, item_drop_model_add_collider_system,
    item_drop_model_system, login_connection_system, login_state_enter_system,
    login_state_exit_system, login_system, model_viewer_enter_system, model_viewer_system,
    npc_idle_sound_system, npc_model_add_collider_system, npc_model_system,
    particle_sequence_system, passive_recovery_system, pending_damage_system,
    pending_skill_effect_system, player_command_system, projectile_system, quest_trigger_system,
    spawn_effect_system, spawn_projectile_system, system_func_event_system, update_position_system,
    visible_status_effects_system, world_connection_system, world_time_system, zone_time_system,
    zone_viewer_enter_system, DebugInspectorPlugin,
};
use ui::{
    load_dialog_sprites_system, ui_character_info_system, ui_chatbox_system,
    ui_debug_camera_info_system, ui_debug_client_entity_list_system,
    ui_debug_command_viewer_system, ui_debug_diagnostics_system, ui_debug_dialog_list_system,
    ui_debug_entity_inspector_system, ui_debug_item_list_system, ui_debug_menu_system,
    ui_debug_npc_list_system, ui_debug_physics_system, ui_debug_render_system,
    ui_debug_skill_list_system, ui_debug_zone_lighting_system, ui_debug_zone_list_system,
    ui_debug_zone_time_system, ui_drag_and_drop_system, ui_game_menu_system, ui_hotbar_system,
    ui_inventory_system, ui_minimap_system, ui_npc_store_system, ui_party_system,
    ui_player_info_system, ui_quest_list_system, ui_selected_target_system, ui_settings_system,
    ui_skill_list_system, widgets::Dialog, DialogLoader, UiStateDebugWindows, UiStateDragAndDrop,
    UiStateWindows,
};
use vfs_asset_io::VfsAssetIo;
use zmo_asset_loader::{ZmoAsset, ZmoAssetLoader};
use zms_asset_loader::{ZmsAssetLoader, ZmsMaterialNumFaces};
use zone_loader::{zone_loader_system, ZoneLoader, ZoneLoaderAsset};

use crate::components::SoundCategory;

pub struct VfsResource {
    vfs: Arc<VfsIndex>,
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
                .help("Runs as zone viewer, loading the specified zone")
                .takes_value(true),
        )
        .arg(
            clap::Arg::new("zone-viewer")
                .long("zone-viewer")
                .help("Run zone viewer"),
        )
        .arg(
            clap::Arg::new("model-viewer")
                .long("model-viewer")
                .help("Run model viewer"),
        )
        .arg(clap::Arg::new("game").long("game").help("Run game"))
        .arg(
            clap::Arg::new("disable-vsync")
                .long("disable-vsync")
                .help("Disable v-sync to see accurate frame times"),
        )
        .arg(
            clap::Arg::new("ip")
                .long("ip")
                .help("Server IP for game login")
                .takes_value(true),
        )
        .arg(
            clap::Arg::new("port")
                .long("port")
                .help("Server port for game login")
                .takes_value(true)
                .default_value("29000"),
        )
        .arg(
            clap::Arg::new("username")
                .long("username")
                .help("Username for game login")
                .takes_value(true),
        )
        .arg(
            clap::Arg::new("password")
                .long("password")
                .help("Password for game login")
                .takes_value(true),
        )
        .arg(
            clap::Arg::new("server-id")
                .long("server-id")
                .help("Server id to use for auto-login")
                .takes_value(true),
        )
        .arg(
            clap::Arg::new("channel-id")
                .long("channel-id")
                .help("Channel id to use for auto-login")
                .takes_value(true),
        )
        .arg(
            clap::Arg::new("character-name")
                .long("character-name")
                .help("If --auto-login is set, this will also auto login to the given character")
                .takes_value(true),
        )
        .arg(
            clap::Arg::new("auto-login")
                .long("auto-login")
                .help("Automatically login to server"),
        )
        .arg(
            clap::Arg::new("passthrough-terrain-textures")
                .long("passthrough-terrain-textures")
                .help("Assume all terrain textures are the same format such that we can pass through compressed textures to the GPU without decompression on the CPU. Note: This is not true for default irose 129_129en assets."),
        )
        .arg(
            clap::Arg::new("enable-sound")
                .long("enable-sound")
                .help("Enable sound."),
        )
        .arg(
            clap::Arg::new("protocol")
            .long("protocol")
            .takes_value(true)
                .value_parser(["irose", "narose667"])
                .default_value("irose")
                .help("Select which protocol to use."),
        );
    let data_path_error = command.error(
        clap::ErrorKind::ArgumentNotFound,
        "Must specify at least one of --data-idx or --data-path",
    );
    let matches = command.get_matches();

    let ip = matches
        .value_of("ip")
        .map(|x| x.to_string())
        .unwrap_or_else(|| "127.0.0.1".to_string());
    let port = matches
        .value_of("ip")
        .map(|x| x.to_string())
        .unwrap_or_else(|| "29000".to_string());
    let preset_username = matches.value_of("username").map(|x| x.to_string());
    let preset_password = matches.value_of("password").map(|x| x.to_string());
    let preset_server_id = matches
        .value_of("server-id")
        .and_then(|x| x.parse::<usize>().ok());
    let preset_channel_id = matches
        .value_of("channel-id")
        .and_then(|x| x.parse::<usize>().ok());
    let preset_character_name = matches.value_of("character-name").map(|x| x.to_string());
    let auto_login = matches.is_present("auto-login");
    let passthrough_terrain_textures = matches.is_present("passthrough-terrain-textures");
    let enable_sound = matches.is_present("enable-sound");
    let protocol_type = match matches.value_of("protocol") {
        Some("irose") => ProtocolType::Irose,
        Some("narose667") => ProtocolType::Narose667,
        _ => ProtocolType::default(),
    };

    let disable_vsync = matches.is_present("disable-vsync");
    let mut app_state = AppState::ZoneViewer;
    let view_zone_id = matches
        .value_of("zone")
        .and_then(|str| str.parse::<u16>().ok())
        .and_then(ZoneId::new)
        .unwrap_or_else(|| ZoneId::new(1).unwrap());
    if matches.is_present("game") {
        app_state = AppState::GameLogin;
    } else if matches.is_present("model-viewer") {
        app_state = AppState::ModelViewer;
    } else if matches.is_present("zone-viewer") {
        app_state = AppState::ZoneViewer;
    }

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
        .insert_resource(WindowDescriptor {
            title: "rose-offline-client".to_string(),
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
        .insert_resource(LogSettings {
            level: Level::INFO,
            filter: "wgpu=error,packets=debug,quest=trace,lua=trace,animation=info".to_string(),
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
        .add_plugin(bevy::window::WindowPlugin::default());

    app.insert_resource(VfsResource { vfs: vfs.clone() })
        .insert_resource(AssetServer::new(VfsAssetIo::new(vfs)))
        .add_plugin(bevy::asset::AssetPlugin::default());

    app.add_plugin(bevy::scene::ScenePlugin::default())
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
        .init_asset_loader::<ZmoAssetLoader>()
        .add_asset::<ZoneLoaderAsset>()
        .init_asset_loader::<DialogLoader>()
        .add_asset::<Dialog>()
        .insert_resource(RenderConfiguration {
            passthrough_terrain_textures,
        })
        .add_plugin(RoseRenderPlugin)
        .add_plugin(RoseScriptingPlugin)
        .insert_resource(ServerConfiguration {
            ip,
            port,
            preset_username,
            preset_password,
            preset_server_id,
            preset_channel_id,
            preset_character_name,
            auto_login,
        })
        .insert_resource(SoundSettings {
            enabled: enable_sound,
            global_gain: 0.6,
            gains: enum_map! {
                SoundCategory::BackgroundMusic => 0.15,
                SoundCategory::PlayerFootstep => 0.9,
                SoundCategory::PlayerCombat => 1.0,
                SoundCategory::OtherFootstep => 0.5,
                SoundCategory::OtherCombat => 0.5,
                SoundCategory::NpcSounds => 0.6,
            },
        });

    // Setup state
    app.add_state(app_state);
    app.add_plugin(DebugInspectorPlugin);

    let mut load_zone_events = Events::<LoadZoneEvent>::default();
    if matches!(app_state, AppState::ZoneViewer) {
        load_zone_events.send(LoadZoneEvent::new(view_zone_id));
    }

    app.insert_resource(load_zone_events)
        .insert_resource(Events::<AnimationFrameEvent>::default())
        .insert_resource(Events::<ChatboxEvent>::default())
        .insert_resource(Events::<ClientEntityEvent>::default())
        .insert_resource(Events::<ConversationDialogEvent>::default())
        .insert_resource(Events::<GameConnectionEvent>::default())
        .insert_resource(Events::<HitEvent>::default())
        .insert_resource(Events::<NpcStoreEvent>::default())
        .insert_resource(Events::<PartyEvent>::default())
        .insert_resource(Events::<PlayerCommandEvent>::default())
        .insert_resource(Events::<QuestTriggerEvent>::default())
        .insert_resource(Events::<SystemFuncEvent>::default())
        .insert_resource(Events::<SpawnEffectEvent>::default())
        .insert_resource(Events::<SpawnProjectileEvent>::default())
        .insert_resource(Events::<WorldConnectionEvent>::default())
        .insert_resource(Events::<ZoneEvent>::default());

    app.add_asset::<ZmsMaterialNumFaces>();

    app.add_system(background_music_system)
        .add_system(character_model_system)
        .add_system(character_model_add_collider_system.after(character_model_system))
        .add_system(npc_model_system)
        .add_system(npc_model_add_collider_system.after(npc_model_system))
        .add_system(item_drop_model_system)
        .add_system(item_drop_model_add_collider_system.after(item_drop_model_system))
        .add_system(animation_system)
        .add_system(particle_sequence_system)
        .add_system(effect_system)
        .add_system(
            animation_effect_system
                .after(animation_system)
                .before(spawn_effect_system),
        )
        .add_system(animation_sound_system.after(animation_system))
        .add_system(pending_skill_effect_system.after(animation_effect_system))
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
            hit_event_system
                .after(animation_effect_system)
                .after(projectile_system),
        )
        .add_system(
            damage_digit_render_system
                .after(pending_damage_system)
                .after(hit_event_system),
        )
        .add_system(update_ui_resources)
        .add_system(spawn_effect_system)
        .add_system(npc_idle_sound_system)
        .add_system(world_time_system)
        .add_system(system_func_event_system)
        .add_system(load_dialog_sprites_system)
        .add_system(zone_time_system.after(world_time_system))
        .add_system(ui_npc_store_system.label("ui_system"))
        .add_system(ui_debug_camera_info_system.label("ui_system"))
        .add_system(ui_debug_client_entity_list_system.label("ui_system"))
        .add_system(ui_debug_command_viewer_system.label("ui_system"))
        .add_system(ui_debug_dialog_list_system.label("ui_system"))
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

    // character_model_blink_system must be in a stage after character_model_system
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
    .add_system_set(SystemSet::on_update(AppState::GameLogin).with_system(login_system));

    // Game Character Select
    app.add_system_set(
        SystemSet::on_enter(AppState::GameCharacterSelect)
            .with_system(character_select_enter_system),
    )
    .add_system_set(
        SystemSet::on_update(AppState::GameCharacterSelect)
            .with_system(character_select_system)
            .with_system(character_select_input_system)
            .with_system(character_select_models_system),
    )
    .add_system_set(
        SystemSet::on_exit(AppState::GameCharacterSelect).with_system(character_select_exit_system),
    );

    // Game
    app.insert_resource(UiStateDragAndDrop::default())
        .insert_resource(UiStateWindows::default())
        .insert_resource(UiStateDebugWindows::default())
        .insert_resource(ClientEntityList::default())
        .insert_resource(DebugRenderConfig::default())
        .insert_resource(WorldTime::default())
        .insert_resource(ZoneTime::default());

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
                .with_system(game_mouse_input_system.after("ui_system"))
                .with_system(
                    player_command_system
                        .after(cooldown_system)
                        .after(game_mouse_input_system),
                )
                .with_system(ui_chatbox_system.label("ui_system"))
                .with_system(ui_character_info_system.label("ui_system"))
                .with_system(ui_inventory_system.label("ui_system"))
                .with_system(ui_game_menu_system.label("ui_system"))
                .with_system(ui_hotbar_system.label("ui_system"))
                .with_system(ui_minimap_system.label("ui_system"))
                .with_system(ui_party_system.label("ui_system"))
                .with_system(ui_player_info_system.label("ui_system"))
                .with_system(ui_quest_list_system.label("ui_system"))
                .with_system(ui_selected_target_system.label("ui_system"))
                .with_system(ui_skill_list_system.label("ui_system"))
                .with_system(ui_settings_system.label("ui_system"))
                .with_system(conversation_dialog_system.label("ui_system")),
        );
    app.add_system_to_stage(CoreStage::PostUpdate, ui_drag_and_drop_system);

    // Setup network
    let (network_thread_tx, network_thread_rx) =
        tokio::sync::mpsc::unbounded_channel::<NetworkThreadMessage>();
    let network_thread = std::thread::spawn(move || run_network_thread(network_thread_rx));
    app.insert_resource(NetworkThread::new(protocol_type, network_thread_tx.clone()));

    // Run network systems before Update, so we can add/remove entities
    app.add_stage_before(
        CoreStage::Update,
        GameStages::Network,
        SystemStage::parallel()
            .with_system(login_connection_system)
            .with_system(world_connection_system)
            .with_system(game_connection_system),
    );

    app.add_startup_system(load_game_data)
        .add_startup_system(load_ui_resources);
    app.run();

    network_thread_tx.send(NetworkThreadMessage::Exit).ok();
    network_thread.join().ok();
}

fn load_game_data(
    mut commands: Commands,
    vfs_resource: Res<VfsResource>,
    asset_server: Res<AssetServer>,
    mut egui_context: ResMut<EguiContext>,
    mut damage_digit_materials: ResMut<Assets<DamageDigitMaterial>>,
) {
    let item_database = Arc::new(
        rose_data_irose::get_item_database(&vfs_resource.vfs)
            .expect("Failed to load item database"),
    );
    let npc_database = Arc::new(
        rose_data_irose::get_npc_database(
            &vfs_resource.vfs,
            &NpcDatabaseOptions {
                load_frame_data: false,
            },
        )
        .expect("Failed to load npc database"),
    );
    let skill_database = Arc::new(
        rose_data_irose::get_skill_database(&vfs_resource.vfs)
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
        rose_data_irose::get_zone_list(&vfs_resource.vfs).expect("Failed to load zone list"),
    );

    asset_server.add_loader(ZoneLoader {
        zone_list: zone_list.clone(),
    });

    commands.insert_resource(GameData {
        ability_value_calculator: rose_game_irose::data::get_ability_value_calculator(
            item_database.clone(),
            skill_database.clone(),
            npc_database.clone(),
        ),
        animation_event_flags: rose_data_irose::get_animation_event_flags(),
        character_motion_database: character_motion_database.clone(),
        data_decoder: rose_data_irose::get_data_decoder(),
        effect_database: rose_data_irose::get_effect_database(&vfs_resource.vfs)
            .expect("Failed to load effect database"),
        items: item_database.clone(),
        npcs: npc_database.clone(),
        quests: Arc::new(
            rose_data_irose::get_quest_database(&vfs_resource.vfs)
                .expect("Failed to load quest database"),
        ),
        skills: skill_database,
        skybox: rose_data_irose::get_skybox_database(&vfs_resource.vfs)
            .expect("Failed to load skybox database"),
        sounds: rose_data_irose::get_sound_database(&vfs_resource.vfs)
            .expect("Failed to load sound database"),
        status_effects: Arc::new(
            rose_data_irose::get_status_effect_database(&vfs_resource.vfs)
                .expect("Failed to load status effect database"),
        ),
        zone_list,
        ltb_event: vfs_resource
            .vfs
            .read_file::<LtbFile, _>("3DDATA/EVENT/ULNGTB_CON.LTB")
            .expect("Failed to load event language file"),
        stl_quest: vfs_resource
            .vfs
            .read_file_with::<StlFile, _>(
                "3DDATA/STB/LIST_QUEST_S.STL",
                &StlReadOptions {
                    language_filter: Some(vec![1]),
                },
            )
            .expect("Failed to load quest string file"),

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
    });

    commands.insert_resource(
        ModelLoader::new(
            vfs_resource.vfs.clone(),
            character_motion_database,
            item_database,
            npc_database,
        )
        .expect("Failed to create model loader"),
    );

    commands.spawn_bundle(Camera3dBundle::default());
    commands.insert_resource(AmbientLight {
        color: Color::rgb(1.0, 1.0, 1.0),
        brightness: 0.9,
    });

    // Load icons
    let mut item_pages = Vec::new();
    for i in 1..=14 {
        let image_handle = asset_server.load(&format!("3DDATA/CONTROL/RES/ICON{:02}.DDS", i));
        let texture_id = egui_context.add_image(image_handle.clone_weak());
        item_pages.push((image_handle, texture_id));
    }

    let mut skill_pages = Vec::new();
    for i in 1..=2 {
        let image_handle = asset_server.load(&format!("3DDATA/CONTROL/RES/SKILL{:02}.DDS", i));
        let texture_id = egui_context.add_image(image_handle.clone_weak());
        skill_pages.push((image_handle, texture_id));
    }

    let window_icons_image = asset_server.load("3DDATA/CONTROL/RES/UI21.DDS");
    let window_icons_texture_id = egui_context.add_image(window_icons_image.clone_weak());

    let minimap_player_icon_image = asset_server.load("3DDATA/CONTROL/RES/MINIMAP_ARROW.TGA");
    let minimap_player_icon_texture_id =
        egui_context.add_image(minimap_player_icon_image.clone_weak());

    commands.insert_resource(Icons {
        item_pages,
        skill_pages,
        window_icons_image: (window_icons_image, window_icons_texture_id),
        minimap_player_icon: (minimap_player_icon_image, minimap_player_icon_texture_id),
    });

    commands.insert_resource(DamageDigitsSpawner::load(
        &asset_server,
        &mut damage_digit_materials,
    ));
}
