use rose_game_common::messages::{client::ClientMessage, server::ServerMessage};

use crate::{
    protocol::{GameClient, LoginClient, WorldClient},
    resources::{GameConnection, LoginConnection, WorldConnection},
};

pub enum NetworkThreadMessage {
    RunLoginClient(LoginClient),
    RunWorldClient(WorldClient),
    RunGameClient(GameClient),
    Exit,
}

pub struct NetworkThread {
    pub control_tx: tokio::sync::mpsc::UnboundedSender<NetworkThreadMessage>,
}

impl NetworkThread {
    pub fn new(control_tx: tokio::sync::mpsc::UnboundedSender<NetworkThreadMessage>) -> Self {
        Self { control_tx }
    }

    pub fn connect_login(&self, ip: &str, port: u16) -> LoginConnection {
        let (server_message_tx, server_message_rx) =
            crossbeam_channel::unbounded::<ServerMessage>();
        let (client_message_tx, client_message_rx) =
            tokio::sync::mpsc::unbounded_channel::<ClientMessage>();

        self.control_tx
            .send(NetworkThreadMessage::RunLoginClient(LoginClient::new(
                format!("{}:{}", ip, port).parse().unwrap(),
                client_message_rx,
                server_message_tx,
            )))
            .ok();

        LoginConnection::new(client_message_tx, server_message_rx)
    }

    pub fn connect_world(
        &self,
        ip: &str,
        port: u16,
        packet_codec_seed: u32,
        login_token: u32,
        password_md5: String,
    ) -> WorldConnection {
        let (server_message_tx, server_message_rx) =
            crossbeam_channel::unbounded::<ServerMessage>();
        let (client_message_tx, client_message_rx) =
            tokio::sync::mpsc::unbounded_channel::<ClientMessage>();

        self.control_tx
            .send(NetworkThreadMessage::RunWorldClient(WorldClient::new(
                format!("{}:{}", ip, port).parse().unwrap(),
                packet_codec_seed,
                client_message_rx,
                server_message_tx,
            )))
            .ok();

        WorldConnection::new(
            client_message_tx,
            server_message_rx,
            login_token,
            password_md5,
        )
    }

    #[allow(dead_code)]
    pub fn connect_game(
        &self,
        ip: &str,
        port: u16,
        packet_codec_seed: u32,
        login_token: u32,
        password_md5: String,
    ) -> GameConnection {
        let (server_message_tx, server_message_rx) =
            crossbeam_channel::unbounded::<ServerMessage>();
        let (client_message_tx, client_message_rx) =
            tokio::sync::mpsc::unbounded_channel::<ClientMessage>();

        self.control_tx
            .send(NetworkThreadMessage::RunGameClient(GameClient::new(
                format!("{}:{}", ip, port).parse().unwrap(),
                packet_codec_seed,
                client_message_rx,
                server_message_tx,
            )))
            .ok();

        GameConnection::new(
            client_message_tx,
            server_message_rx,
            login_token,
            password_md5,
        )
    }
}

pub fn run_network_thread(
    mut control_rx: tokio::sync::mpsc::UnboundedReceiver<NetworkThreadMessage>,
) {
    loop {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                loop {
                    match control_rx.recv().await {
                        Some(NetworkThreadMessage::RunLoginClient(mut client)) => {
                            tokio::spawn(async move {
                                client.run_connection().await.ok();
                            });
                        }
                        Some(NetworkThreadMessage::RunWorldClient(mut client)) => {
                            tokio::spawn(async move {
                                client.run_connection().await.ok();
                            });
                        }
                        Some(NetworkThreadMessage::RunGameClient(mut client)) => {
                            tokio::spawn(async move {
                                client.run_connection().await.ok();
                            });
                        }
                        Some(NetworkThreadMessage::Exit) => return,
                        None => return,
                    }
                }
            })
    }
}
