use num_traits::FromPrimitive;
use rose_game_common::messages::{
    client::{ClientMessage, GetChannelList, JoinServer, LoginRequest},
    server::{
        ChannelList, ChannelListError, ConnectionRequestError, ConnectionResponse, JoinServerError,
        JoinServerResponse, LoginError, LoginResponse, ServerMessage,
    },
};
use std::net::SocketAddr;
use thiserror::Error;
use tokio::net::TcpStream;

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

#[derive(Debug, Error)]
pub enum LoginClientError {
    #[error("client initiated disconnect")]
    ClientInitiatedDisconnect,
}

pub struct LoginClient {
    packet_codec: Box<dyn PacketCodec + Send + Sync>,
    client_message_rx: tokio::sync::mpsc::UnboundedReceiver<ClientMessage>,
    server_message_tx: crossbeam_channel::Sender<ServerMessage>,
}

impl LoginClient {
    // TODO: Pass irose into this
    pub fn new(
        client_message_rx: tokio::sync::mpsc::UnboundedReceiver<ClientMessage>,
        server_message_tx: crossbeam_channel::Sender<ServerMessage>,
    ) -> Self {
        Self {
            packet_codec: Box::new(ClientPacketCodec::default(&IROSE_112_TABLE)),
            client_message_rx,
            server_message_tx,
        }
    }

    async fn handle_packet(&self, packet: Packet) -> Result<(), anyhow::Error> {
        match FromPrimitive::from_u16(packet.command) {
            Some(ServerPackets::NetworkStatus) => {
                let response = PacketConnectionReply::try_from(&packet)?;
                let message = match response.status {
                    ConnectionResult::Accepted => Ok(ConnectionResponse {
                        packet_sequence_id: response.packet_sequence_id,
                    }),
                    _ => Err(ConnectionRequestError::Failed),
                };
                self.server_message_tx
                    .send(ServerMessage::ConnectionResponse(message))
                    .ok();
            }
            Some(ServerPackets::LoginReply) => {
                let response = PacketServerLoginReply::try_from(&packet)?;
                let message = match response.result {
                    LoginResult::Ok => Ok(LoginResponse {
                        server_list: response.servers,
                    }),
                    LoginResult::UnknownAccount => Err(LoginError::InvalidAccount),
                    LoginResult::InvalidPassword => Err(LoginError::InvalidPassword),
                    LoginResult::AlreadyLoggedIn => Err(LoginError::AlreadyLoggedIn),
                    _ => Err(LoginError::Failed),
                };
                self.server_message_tx
                    .send(ServerMessage::LoginResponse(message))
                    .ok();
            }
            Some(ServerPackets::ChannelList) => {
                let response = PacketServerChannelList::try_from(&packet)?;
                if response.channels.is_empty() {
                    self.server_message_tx
                        .send(ServerMessage::ChannelList(Err(
                            ChannelListError::InvalidServerId(response.server_id),
                        )))
                        .ok();
                } else {
                    let mut channels = Vec::with_capacity(response.channels.len());
                    for channel in response.channels {
                        channels.push((channel.id, channel.name.to_string()));
                    }
                    self.server_message_tx
                        .send(ServerMessage::ChannelList(Ok(ChannelList {
                            server_id: response.server_id,
                            channels,
                        })))
                        .ok();
                }
            }
            Some(ServerPackets::SelectServer) => {
                let response = PacketServerSelectServer::try_from(&packet)?;
                let message = match response.result {
                    SelectServerResult::Ok => Ok(JoinServerResponse {
                        login_token: response.login_token,
                        packet_codec_seed: response.packet_codec_seed,
                        ip: response.ip.into(),
                        port: response.port,
                    }),
                    SelectServerResult::InvalidChannel => Err(JoinServerError::InvalidChannelId),
                    _ => Err(JoinServerError::InvalidServerId),
                };
                self.server_message_tx
                    .send(ServerMessage::JoinServer(message))
                    .ok();
            }
            _ => println!("Unhandled packet {}", packet.command),
        }

        Ok(())
    }

    async fn handle_client_message(
        &self,
        connection: &mut Connection<'_>,
        message: ClientMessage,
    ) -> Result<(), anyhow::Error> {
        match message {
            ClientMessage::ConnectionRequest(_) => {
                connection
                    .write_packet(Packet::from(&PacketClientConnect {}))
                    .await?
            }
            ClientMessage::LoginRequest(LoginRequest {
                username,
                password_md5,
            }) => {
                connection
                    .write_packet(Packet::from(&PacketClientLoginRequest {
                        username: &username,
                        password_md5: &password_md5,
                    }))
                    .await?
            }
            ClientMessage::GetChannelList(GetChannelList { server_id }) => {
                connection
                    .write_packet(Packet::from(&PacketClientChannelList { server_id }))
                    .await?
            }
            ClientMessage::JoinServer(JoinServer {
                server_id,
                channel_id,
            }) => {
                connection
                    .write_packet(Packet::from(&PacketClientSelectServer {
                        server_id,
                        channel_id,
                    }))
                    .await?
            }
            _ => {
                println!("TODO: Send client message {:?}", message);
            }
        }
        Ok(())
    }

    pub async fn run_connection(&mut self, address: SocketAddr) -> Result<(), anyhow::Error> {
        let socket = TcpStream::connect(&address).await?;
        let mut connection = Connection::new(socket, self.packet_codec.as_ref());

        loop {
            tokio::select! {
                packet = connection.read_packet() => {
                    match packet {
                        Ok(packet) => {
                            self.handle_packet(packet).await?;
                        },
                        Err(error) => {
                            return Err(error);
                        }
                    }
                },
                server_message = self.client_message_rx.recv() => {
                    if let Some(message) = server_message {
                        self.handle_client_message(&mut connection, message).await?;
                    } else {
                        return Err(LoginClientError::ClientInitiatedDisconnect.into());
                    }
                }
            };
        }

        // Ok(())
    }
}
