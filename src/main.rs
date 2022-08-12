use std::path::Path;

use rose_data::ZoneId;
use rose_offline_client::{
    load_config, run_game, run_model_viewer, run_zone_viewer, Config, FilesystemDeviceConfig,
    SystemsConfig,
};

fn main() {
    let command = clap::Command::new("rose-offline-client")
        .arg(
            clap::Arg::new("config")
                .long("config")
                .help("Path to config.toml")
                .takes_value(true),
        )
        .arg(
            clap::Arg::new("data-idx")
                .long("data-idx")
                .help("Path to data.idx")
                .takes_value(true),
        )
        .arg(
            clap::Arg::new("data-aruavfs-idx")
                .long("data-aruavfs-idx")
                .help("Path to aruarose data.idx")
                .takes_value(true),
        )
        .arg(
            clap::Arg::new("data-titanvfs-idx")
                .long("data-titanvfs-idx")
                .help("Path to titanrose data.idx")
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
            clap::Arg::new("disable-sound")
                .long("disable-sound")
                .help("Disable sound."),
        )
        .arg(
            clap::Arg::new("data-version")
            .long("data-version")
            .takes_value(true)
                .value_parser(["irose"])
                .help("Select which game version to use for game data."),
        )
        .arg(
            clap::Arg::new("network-version")
            .long("network-version")
            .takes_value(true)
                .value_parser(["irose"])
                .help("Select which game version to use for network."),
        )
        .arg(
            clap::Arg::new("ui-version")
            .long("ui-version")
            .takes_value(true)
                .value_parser(["irose"])
                .help("Select which game version to use for ui."),
        );
    let matches = command.get_matches();

    let mut config = matches
        .value_of("config")
        .map(Path::new)
        .map_or_else(Config::default, load_config);

    if let Some(ip) = matches.value_of("ip") {
        config.server.ip = ip.into();
    }

    if let Some(port) = matches.value_of("port").and_then(|s| s.parse::<u16>().ok()) {
        config.server.port = port;
    }

    if let Some(username) = matches.value_of("username") {
        config.account.username = username.into();
    }

    if let Some(password) = matches.value_of("password") {
        config.account.password = password.into();
    }

    if matches.is_present("auto-login") {
        config.auto_login.enabled = true;
    }

    if let Some(id) = matches
        .value_of("server-id")
        .and_then(|s| s.parse::<usize>().ok())
    {
        config.auto_login.server_id = Some(id);
    }

    if let Some(id) = matches
        .value_of("channel-id")
        .and_then(|s| s.parse::<usize>().ok())
    {
        config.auto_login.channel_id = Some(id);
    }

    if let Some(character_name) = matches.value_of("character-name") {
        config.auto_login.character_name = Some(character_name.into());
    }

    if matches.is_present("disable-vsync") {
        config.graphics.disable_vsync = true;
    }

    if matches.is_present("passthrough-terrain-textures") {
        config.graphics.passthrough_terrain_textures = true;
    }

    if matches.is_present("disable-sound") {
        config.sound.enabled = false;
    }

    if let Some(version) = matches.value_of("data-version") {
        config.game.data_version = version.to_string();
    }

    if let Some(version) = matches.value_of("network-version") {
        config.game.network_version = version.to_string();
    }

    if let Some(version) = matches.value_of("ui-version") {
        config.game.ui_version = version.to_string();
    }

    if let Some(vfs_path) = matches.value_of("data-idx") {
        config
            .filesystem
            .devices
            .insert(0, FilesystemDeviceConfig::Vfs(vfs_path.into()));
    }

    if let Some(aruavfs_path) = matches.value_of("data-aruavfs-idx") {
        config
            .filesystem
            .devices
            .insert(0, FilesystemDeviceConfig::AruaVfs(aruavfs_path.into()));
    }

    if let Some(titanvfs_path) = matches.value_of("data-titanvfs-idx") {
        config
            .filesystem
            .devices
            .insert(0, FilesystemDeviceConfig::TitanVfs(titanvfs_path.into()));
    }

    if let Some(directory_path) = matches.value_of("data-path") {
        config
            .filesystem
            .devices
            .insert(0, FilesystemDeviceConfig::Directory(directory_path.into()));
    }

    if matches.is_present("model-viewer") {
        run_model_viewer(&config);
    } else if matches.is_present("zone-viewer") {
        run_zone_viewer(
            &config,
            matches
                .value_of("zone")
                .and_then(|str| str.parse::<u16>().ok())
                .and_then(ZoneId::new),
        );
    } else {
        run_game(&config, SystemsConfig::default());
    }
}
