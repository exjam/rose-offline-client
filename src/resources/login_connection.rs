use bevy::prelude::Resource;
use rose_game_common::{
    data::Password,
    messages::{client::ClientMessage, server::ServerMessage},
};

#[derive(Resource)]
pub struct LoginConnection {
    pub client_message_tx: tokio::sync::mpsc::UnboundedSender<ClientMessage>,
    pub server_message_rx: crossbeam_channel::Receiver<ServerMessage>,
}

impl LoginConnection {
    pub fn new(
        client_message_tx: tokio::sync::mpsc::UnboundedSender<ClientMessage>,
        server_message_rx: crossbeam_channel::Receiver<ServerMessage>,
    ) -> Self {
        client_message_tx
            .send(ClientMessage::ConnectionRequest {
                login_token: 0,
                password: Password::Md5(String::default()),
            })
            .ok();

        Self {
            client_message_tx,
            server_message_rx,
        }
    }
}
