use bevy::prelude::Event;

#[derive(Event)]
pub enum LoginEvent {
    Login { username: String, password: String },
    SelectServer { server_id: usize, channel_id: usize },
}
