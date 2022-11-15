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
    pub world_servers: Vec<ServerListWorldServer>,
}
