use rose_game_common::messages::{client::ClientMessage, server::ServerMessage};

#[cfg(not(target_arch = "wasm32"))]
pub struct LoginConnection {
    pub client_message_tx: tokio::sync::mpsc::UnboundedSender<ClientMessage>,
    pub server_message_rx: crossbeam_channel::Receiver<ServerMessage>,
}

#[cfg(not(target_arch = "wasm32"))]
impl LoginConnection {
    pub fn new(
        client_message_tx: tokio::sync::mpsc::UnboundedSender<ClientMessage>,
        server_message_rx: crossbeam_channel::Receiver<ServerMessage>,
    ) -> Self {
        client_message_tx
            .send(ClientMessage::ConnectionRequest(Default::default()))
            .ok();

        Self {
            client_message_tx,
            server_message_rx,
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub struct LoginConnection {
    pub client_message_tx: crossbeam_channel::Sender<ClientMessage>,
    pub server_message_rx: crossbeam_channel::Receiver<ServerMessage>,
}
