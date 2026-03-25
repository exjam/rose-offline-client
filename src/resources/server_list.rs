use bevy::prelude::Resource;

pub struct ServerListGameServer {
    pub id: usize,
    pub name: String,
}

pub struct ServerListWorldServer {
    pub id: usize,
    pub name: String,
    pub game_servers: Vec<ServerListGameServer>,
}

#[derive(Resource)]
pub struct ServerList {
    pub selected_server: Option<usize>,
    pub selected_channel: Option<usize>,
    pub world_servers: Vec<ServerListWorldServer>,
}

impl From<Vec<ServerListWorldServer>> for ServerList {
    fn from(value: Vec<ServerListWorldServer>) -> Self {
        ServerList {
            selected_server: None,
            selected_channel: None,
            world_servers: value,
        }
    }
}
