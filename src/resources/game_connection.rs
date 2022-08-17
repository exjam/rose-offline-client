use rose_game_common::{
    data::Password,
    messages::{
        client::{ClientMessage, ConnectionRequest},
        server::ServerMessage,
    },
};

#[cfg(not(target_arch = "wasm32"))]
pub struct GameConnection {
    pub client_message_tx: tokio::sync::mpsc::UnboundedSender<ClientMessage>,
    pub server_message_rx: crossbeam_channel::Receiver<ServerMessage>,
}

#[cfg(not(target_arch = "wasm32"))]
impl GameConnection {
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

#[cfg(target_arch = "wasm32")]
pub struct GameConnection {
    pub client_message_tx: crossbeam_channel::Sender<ClientMessage>,
    pub server_message_rx: crossbeam_channel::Receiver<ServerMessage>,
}
