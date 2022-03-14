pub struct ServerConfiguration {
    pub ip: String,
    pub port: String,
    pub preset_username: Option<String>,
    pub preset_password: Option<String>,
    pub preset_server_id: Option<usize>,
    pub preset_channel_id: Option<usize>,
    pub preset_character_name: Option<String>,
    pub auto_login: bool,
}
