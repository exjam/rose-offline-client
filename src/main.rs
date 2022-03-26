#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

use bevy::{
    asset::AssetServerSettings,
    core_pipeline::ClearColor,
    ecs::{event::Events, schedule::ShouldRun},
    log::{Level, LogSettings},
    prelude::{
        AddAsset, App, AssetServer, Color, Commands, CoreStage, Msaa,
        ParallelSystemDescriptorCoercion, PerspectiveCameraBundle, Res, StageLabel, State,
        SystemSet, SystemStage,
    },
    render::{render_resource::WgpuFeatures, settings::WgpuSettings},
    window::WindowDescriptor,
};
use std::{path::Path, sync::Arc};

mod components;
mod events;
mod fly_camera;
mod follow_camera;
mod model_loader;
mod protocol;
mod render;
mod resources;
mod systems;
mod vfs_asset_io;
mod zmo_asset_loader;
mod zms_asset_loader;

use rose_data::{CharacterMotionDatabaseOptions, NpcDatabaseOptions, ZoneId};
use rose_file_readers::VfsIndex;

use events::{ChatboxEvent, GameConnectionEvent, LoadZoneEvent, WorldConnectionEvent, ZoneEvent};
use fly_camera::FlyCameraPlugin;
use follow_camera::FollowCameraPlugin;
use model_loader::ModelLoader;
use render::RoseRenderPlugin;
use resources::{
    run_network_thread, AppState, GameData, NetworkThread, NetworkThreadMessage,
    ServerConfiguration,
};
use systems::{
    ability_values_system, animation_system, character_model_add_collider_system,
    character_model_system, character_select_enter_system, character_select_exit_system,
    character_select_models_system, character_select_system, collision_add_colliders_system,
    collision_system, command_system, debug_render_collider_system, debug_render_skeleton_system,
    diagnostics_ui_system, game_connection_system, game_debug_ui_system, game_input_system,
    game_state_enter_system, game_ui_system, game_zone_change_system, load_zone_system,
    login_connection_system, login_state_enter_system, login_state_exit_system, login_system,
    model_viewer_enter_system, model_viewer_system, npc_model_add_collider_system,
    npc_model_system, update_position_system, world_connection_system, zone_viewer_setup_system,
    zone_viewer_system, DebugInspectorPlugin,
};
use vfs_asset_io::VfsAssetIo;
use zmo_asset_loader::{ZmoAsset, ZmoAssetLoader};
use zms_asset_loader::ZmsAssetLoader;

pub struct VfsResource {
    vfs: Arc<VfsIndex>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, StageLabel)]
enum GameStages {
    Network,
    ZoneChange,
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

    let disable_vsync = matches.is_present("disable-vsync");
    let mut app_state = AppState::ZoneViewer;
    let view_zone_id = matches
        .value_of("zone")
        .and_then(|str| str.parse::<u16>().ok())
        .and_then(ZoneId::new)
        .unwrap_or_else(|| ZoneId::new(2).unwrap());
    if matches.is_present("game") {
        app_state = AppState::GameLogin;
    } else if matches.is_present("model-viewer") {
        app_state = AppState::ModelViewer;
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
        .insert_resource(LogSettings {
            level: Level::INFO,
            filter: "wgpu=error,packets=trace".to_string(),
        })
        .add_plugin(bevy::log::LogPlugin::default())
        .add_plugin(bevy::core::CorePlugin::default())
        .add_plugin(bevy::diagnostic::EntityCountDiagnosticsPlugin::default())
        .add_plugin(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default())
        .add_plugin(bevy::transform::TransformPlugin::default())
        .add_plugin(bevy::hierarchy::HierarchyPlugin::default())
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
    app.add_plugin(bevy_polyline::PolylinePlugin)
        .add_plugin(bevy_egui::EguiPlugin)
        .add_plugin(smooth_bevy_cameras::LookTransformPlugin)
        .add_plugin(bevy_rapier3d::physics::RapierPhysicsPlugin::<
            bevy_rapier3d::physics::NoUserData,
        >::default())
        .insert_resource(bevy_rapier3d::physics::RapierConfiguration {
            physics_pipeline_active: false,
            query_pipeline_active: true,
            ..Default::default()
        });

    // Initialise rose stuff
    app.init_asset_loader::<ZmsAssetLoader>()
        .add_asset::<ZmoAsset>()
        .init_asset_loader::<ZmoAssetLoader>()
        .add_plugin(FlyCameraPlugin::default())
        .add_plugin(FollowCameraPlugin::default())
        .add_plugin(RoseRenderPlugin)
        .insert_resource(ServerConfiguration {
            ip,
            port,
            preset_username,
            preset_password,
            preset_server_id,
            preset_channel_id,
            preset_character_name,
            auto_login,
        });

    // Setup state
    app.add_state(app_state);
    app.add_plugin(DebugInspectorPlugin);

    let mut load_zone_events = Events::<LoadZoneEvent>::default();
    if matches!(app_state, AppState::ZoneViewer) {
        load_zone_events.send(LoadZoneEvent::new(view_zone_id));
    }

    app.insert_resource(Events::<ChatboxEvent>::default())
        .insert_resource(load_zone_events)
        .insert_resource(Events::<ZoneEvent>::default())
        .insert_resource(Events::<GameConnectionEvent>::default())
        .insert_resource(Events::<WorldConnectionEvent>::default());

    app.add_system(character_model_system.label("character_model_system"))
        .add_system(character_model_add_collider_system.after("character_model_system"))
        .add_system(npc_model_system.label("npc_model_system"))
        .add_system(npc_model_add_collider_system.after("npc_model_system"))
        .add_system(debug_render_collider_system)
        .add_system(debug_render_skeleton_system)
        .add_system(collision_system)
        .add_system(collision_add_colliders_system)
        .add_system(animation_system.label("animation_system"))
        .add_system(diagnostics_ui_system);

    // Run zone change system after Update, so we do can add/remove entities
    app.add_stage_after(
        CoreStage::Update,
        GameStages::ZoneChange,
        SystemStage::parallel()
            .with_system(load_zone_system)
            .with_system(game_zone_change_system),
    );

    // Zone Viewer
    app.add_system_set(
        SystemSet::on_enter(AppState::ZoneViewer).with_system(zone_viewer_setup_system),
    )
    .add_system_set(SystemSet::on_update(AppState::ZoneViewer).with_system(zone_viewer_system));

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
            .with_system(character_select_models_system),
    )
    .add_system_set(
        SystemSet::on_exit(AppState::GameCharacterSelect).with_system(character_select_exit_system),
    );

    // Game
    app.add_system_set(SystemSet::on_enter(AppState::Game).with_system(game_state_enter_system))
        .add_system_set(
            SystemSet::on_update(AppState::Game)
                .with_system(ability_values_system)
                .with_system(command_system.after("animation_system"))
                .with_system(update_position_system)
                .with_system(game_ui_system.label("game_ui_system"))
                .with_system(
                    game_debug_ui_system
                        .label("game_debug_ui_system")
                        .after("game_ui_system"),
                )
                .with_system(game_input_system.after("game_debug_ui_system")),
        );

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

    app.add_startup_system(load_game_data);
    app.run();

    network_thread_tx.send(NetworkThreadMessage::Exit).ok();
    network_thread.join().ok();
}

fn load_game_data(mut commands: Commands, vfs_resource: Res<VfsResource>) {
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

    commands.insert_resource(GameData {
        ability_value_calculator: rose_game_irose::data::get_ability_value_calculator(
            item_database.clone(),
            skill_database.clone(),
            npc_database.clone(),
        ),
        character_motion_database: character_motion_database.clone(),
        data_decoder: rose_data_irose::get_data_decoder(),
        items: item_database.clone(),
        npcs: npc_database.clone(),
        quests: Arc::new(
            rose_data_irose::get_quest_database(&vfs_resource.vfs)
                .expect("Failed to load quest database"),
        ),
        skills: skill_database,
        status_effects: Arc::new(
            rose_data_irose::get_status_effect_database(&vfs_resource.vfs)
                .expect("Failed to load status effect database"),
        ),
        zone_list: Arc::new(
            rose_data_irose::get_zone_list(&vfs_resource.vfs).expect("Failed to load zone list"),
        ),
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

    commands.spawn_bundle(PerspectiveCameraBundle::default());
}
