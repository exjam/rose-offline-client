use async_trait::async_trait;
use num_traits::FromPrimitive;
use std::net::SocketAddr;
use tokio::net::TcpStream;

use rose_game_common::messages::{
    client::ClientMessage,
    server::{
        ChannelListError, ConnectionRequestError, JoinServerError, LoginError, ServerMessage,
    },
};
use rose_network_common::{Connection, Packet, PacketCodec};
use rose_network_irose::{
    login_client_packets::{
        PacketClientChannelList, PacketClientConnect, PacketClientLoginRequest,
        PacketClientSelectServer,
    },
    login_server_packets::{
        ConnectionResult, LoginResult, PacketConnectionReply, PacketServerChannelList,
        PacketServerLoginReply, PacketServerSelectServer, SelectServerResult, ServerPackets,
    },
    ClientPacketCodec, IROSE_112_TABLE,
};

use crate::protocol::{ProtocolClient, ProtocolClientError};

pub struct LoginClient {
    server_address: SocketAddr,
    client_message_rx: tokio::sync::mpsc::UnboundedReceiver<ClientMessage>,
    server_message_tx: crossbeam_channel::Sender<ServerMessage>,
    packet_codec: Box<dyn PacketCodec + Send + Sync>,
}

impl LoginClient {
    pub fn new(
        server_address: SocketAddr,
        client_message_rx: tokio::sync::mpsc::UnboundedReceiver<ClientMessage>,
        server_message_tx: crossbeam_channel::Sender<ServerMessage>,
    ) -> Self {
        Self {
            server_address,
            client_message_rx,
            server_message_tx,
            packet_codec: Box::new(ClientPacketCodec::default(&IROSE_112_TABLE)),
        }
    }

    async fn handle_packet(&self, packet: &Packet) -> Result<(), anyhow::Error> {
        match FromPrimitive::from_u16(packet.command) {
            Some(ServerPackets::NetworkStatus) => {
                let response = PacketConnectionReply::try_from(packet)?;
                let message = match response.status {
                    ConnectionResult::Accepted => ServerMessage::ConnectionRequestSuccess {
                        packet_sequence_id: response.packet_sequence_id,
                    },
                    _ => ServerMessage::ConnectionRequestError {
                        error: ConnectionRequestError::Failed,
                    },
                };
                self.server_message_tx.send(message).ok();
            }
            Some(ServerPackets::LoginReply) => {
                let response = PacketServerLoginReply::try_from(packet)?;
                let message = match response.result {
                    LoginResult::Ok => ServerMessage::LoginSuccess {
                        server_list: response.servers,
                    },
                    LoginResult::UnknownAccount => ServerMessage::LoginError {
                        error: LoginError::InvalidAccount,
                    },
                    LoginResult::InvalidPassword => ServerMessage::LoginError {
                        error: LoginError::InvalidPassword,
                    },
                    LoginResult::AlreadyLoggedIn => ServerMessage::LoginError {
                        error: LoginError::AlreadyLoggedIn,
                    },
                    _ => ServerMessage::LoginError {
                        error: LoginError::Failed,
                    },
                };
                self.server_message_tx.send(message).ok();
            }
            Some(ServerPackets::ChannelList) => {
                let response = PacketServerChannelList::try_from(packet)?;
                if response.channels.is_empty() {
                    self.server_message_tx
                        .send(ServerMessage::ChannelListError {
                            error: ChannelListError::InvalidServerId {
                                server_id: response.server_id,
                            },
                        })
                        .ok();
                } else {
                    let mut channels = Vec::with_capacity(response.channels.len());
                    for channel in response.channels {
                        channels.push((channel.id, channel.name.to_string()));
                    }
                    self.server_message_tx
                        .send(ServerMessage::ChannelList {
                            server_id: response.server_id,
                            channels,
                        })
                        .ok();
                }
            }
            Some(ServerPackets::SelectServer) => {
                let response = PacketServerSelectServer::try_from(packet)?;
                let message = match response.result {
                    SelectServerResult::Ok => ServerMessage::JoinServerSuccess {
                        login_token: response.login_token,
                        packet_codec_seed: response.packet_codec_seed,
                        ip: response.ip.into(),
                        port: response.port,
                    },
                    SelectServerResult::InvalidChannel => ServerMessage::JoinServerError {
                        error: JoinServerError::InvalidChannelId,
                    },
                    _ => ServerMessage::JoinServerError {
                        error: JoinServerError::InvalidServerId,
                    },
                };
                self.server_message_tx.send(message).ok();
            }
            _ => log::info!("Unhandled LoginClient packet {:?}", packet),
        }

        Ok(())
    }

    async fn handle_client_message(
        &self,
        connection: &mut Connection<'_>,
        message: ClientMessage,
    ) -> Result<(), anyhow::Error> {
        match message {
            ClientMessage::ConnectionRequest { .. } => {
                connection
                    .write_packet(Packet::from(&PacketClientConnect {}))
                    .await?
            }
            ClientMessage::LoginRequest { username, password } => {
                connection
                    .write_packet(Packet::from(&PacketClientLoginRequest {
                        username: &username,
                        password_md5: &password.to_md5(),
                    }))
                    .await?
            }
            ClientMessage::GetChannelList { server_id } => {
                connection
                    .write_packet(Packet::from(&PacketClientChannelList { server_id }))
                    .await?
            }
            ClientMessage::JoinServer {
                server_id,
                channel_id,
            } => {
                connection
                    .write_packet(Packet::from(&PacketClientSelectServer {
                        server_id,
                        channel_id,
                    }))
                    .await?
            }
            unimplemented => {
                log::info!(
                    "Unimplemented LoginClient ClientMessage {:?}",
                    unimplemented
                );
            }
        }
        Ok(())
    }
}

implement_protocol_client! { LoginClient }
