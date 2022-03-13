pub struct ServerListGameServer {
    pub id: usize,
    pub name: String,
}

pub struct ServerListWorldServer {
    pub id: usize,
    pub name: String,
    pub game_servers: Vec<ServerListGameServer>,
}

pub struct ServerList {
    pub world_servers: Vec<ServerListWorldServer>,
}
