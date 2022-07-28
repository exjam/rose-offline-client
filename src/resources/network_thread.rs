use rose_game_common::messages::{client::ClientMessage, server::ServerMessage};

use crate::{
    protocol::{irose, narose667, ProtocolClient, ProtocolType},
    resources::{GameConnection, LoginConnection, WorldConnection},
};

pub enum NetworkThreadMessage {
    RunProtocolClient(Box<dyn ProtocolClient + Send + Sync>),
    Exit,
}

pub struct NetworkThread {
    pub control_tx: tokio::sync::mpsc::UnboundedSender<NetworkThreadMessage>,
    pub protocol_type: ProtocolType,
}

impl NetworkThread {
    pub fn new(
        protocol_type: ProtocolType,
        control_tx: tokio::sync::mpsc::UnboundedSender<NetworkThreadMessage>,
    ) -> Self {
        Self {
            protocol_type,
            control_tx,
        }
    }

    pub fn connect_login(&self, ip: &str, port: u16) -> LoginConnection {
        let (server_message_tx, server_message_rx) =
            crossbeam_channel::unbounded::<ServerMessage>();
        let (client_message_tx, client_message_rx) =
            tokio::sync::mpsc::unbounded_channel::<ClientMessage>();
        let server_address = format!("{}:{}", ip, port).parse().unwrap();

        let client = match self.protocol_type {
            ProtocolType::Irose => Box::new(irose::LoginClient::new(
                server_address,
                client_message_rx,
                server_message_tx,
            )) as Box<dyn ProtocolClient + Send + Sync>,
            ProtocolType::Narose667 => Box::new(narose667::LoginClient::new(
                server_address,
                client_message_rx,
                server_message_tx,
            )) as Box<dyn ProtocolClient + Send + Sync>,
        };

        self.control_tx
            .send(NetworkThreadMessage::RunProtocolClient(client))
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
        let server_address = format!("{}:{}", ip, port).parse().unwrap();

        let client = match self.protocol_type {
            ProtocolType::Irose => Box::new(irose::WorldClient::new(
                server_address,
                packet_codec_seed,
                client_message_rx,
                server_message_tx,
            )) as Box<dyn ProtocolClient + Send + Sync>,
            ProtocolType::Narose667 => Box::new(narose667::WorldClient::new(
                server_address,
                client_message_rx,
                server_message_tx,
            )) as Box<dyn ProtocolClient + Send + Sync>,
        };

        self.control_tx
            .send(NetworkThreadMessage::RunProtocolClient(client))
            .ok();

        WorldConnection::new(
            client_message_tx,
            server_message_rx,
            login_token,
            password_md5,
        )
    }

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
        let server_address = format!("{}:{}", ip, port).parse().unwrap();

        let client = match self.protocol_type {
            ProtocolType::Irose => Box::new(irose::GameClient::new(
                server_address,
                packet_codec_seed,
                client_message_rx,
                server_message_tx,
            )) as Box<dyn ProtocolClient + Send + Sync>,
            ProtocolType::Narose667 => Box::new(narose667::GameClient::new(
                server_address,
                client_message_rx,
                server_message_tx,
            )) as Box<dyn ProtocolClient + Send + Sync>,
        };

        self.control_tx
            .send(NetworkThreadMessage::RunProtocolClient(client))
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
                        Some(NetworkThreadMessage::RunProtocolClient(mut client)) => {
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
