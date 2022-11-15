use bevy::prelude::Resource;
use rose_game_common::{
    data::Password,
    messages::{
        client::{ClientMessage, ConnectionRequest},
        server::ServerMessage,
    },
};

#[derive(Resource)]
pub struct WorldConnection {
    pub client_message_tx: tokio::sync::mpsc::UnboundedSender<ClientMessage>,
    pub server_message_rx: crossbeam_channel::Receiver<ServerMessage>,
}

impl WorldConnection {
    pub fn new(
        client_message_tx: tokio::sync::mpsc::UnboundedSender<ClientMessage>,
        server_message_rx: crossbeam_channel::Receiver<ServerMessage>,
        login_token: u32,
        password: Password,
    ) -> Self {
        client_message_tx
            .send(ClientMessage::ConnectionRequest(ConnectionRequest {
                login_token,
                password,
            }))
            .ok();

        Self {
            client_message_tx,
            server_message_rx,
        }
    }
}
