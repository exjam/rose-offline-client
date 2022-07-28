use async_trait::async_trait;
use num_traits::FromPrimitive;
use std::net::SocketAddr;
use tokio::net::TcpStream;

use rose_game_common::messages::{
    client::{ClientMessage, ConnectionRequest, CreateCharacter},
    server::{
        ConnectionRequestError, ConnectionResponse, CreateCharacterError, CreateCharacterResponse,
        JoinServerResponse, ServerMessage,
    },
};
use rose_network_common::{Connection, Packet, PacketCodec};
use rose_network_narose667::{
    world_client_packets::{
        PacketClientCharacterList, PacketClientConnectRequest, PacketClientCreateCharacter,
        PacketClientSelectCharacter,
    },
    world_server_packets::{
        ConnectResult, CreateCharacterResult, PacketConnectionReply, PacketServerCharacterList,
        PacketServerCreateCharacterReply, PacketServerMoveServer, ServerPackets,
    },
    ClientPacketCodec,
};

use crate::protocol::{ProtocolClient, ProtocolClientError};
pub struct WorldClient {
    server_address: SocketAddr,
    client_message_rx: tokio::sync::mpsc::UnboundedReceiver<ClientMessage>,
    server_message_tx: crossbeam_channel::Sender<ServerMessage>,
    packet_codec: Box<dyn PacketCodec + Send + Sync>,
}

impl WorldClient {
    pub fn new(
        server_address: SocketAddr,
        client_message_rx: tokio::sync::mpsc::UnboundedReceiver<ClientMessage>,
        server_message_tx: crossbeam_channel::Sender<ServerMessage>,
    ) -> Self {
        Self {
            server_address,
            client_message_rx,
            server_message_tx,
            packet_codec: Box::new(ClientPacketCodec::default()),
        }
    }

    async fn handle_packet(&self, packet: &Packet) -> Result<(), anyhow::Error> {
        match FromPrimitive::from_u16(packet.command) {
            Some(ServerPackets::ConnectReply) => {
                let response = PacketConnectionReply::try_from(packet)?;
                let message = match response.result {
                    ConnectResult::Ok => Ok(ConnectionResponse {
                        packet_sequence_id: response.packet_sequence_id,
                    }),
                    _ => Err(ConnectionRequestError::Failed),
                };
                self.server_message_tx
                    .send(ServerMessage::ConnectionResponse(message))
                    .ok();
            }
            Some(ServerPackets::CharacterListReply) => {
                self.server_message_tx
                    .send(ServerMessage::CharacterList(
                        PacketServerCharacterList::try_from(packet)?.characters,
                    ))
                    .ok();
            }
            Some(ServerPackets::MoveServer) => {
                let response = PacketServerMoveServer::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::SelectCharacter(Ok(JoinServerResponse {
                        login_token: response.login_token,
                        packet_codec_seed: response.packet_codec_seed,
                        ip: response.ip.to_string(),
                        port: response.port,
                    })))
                    .ok();
            }
            Some(ServerPackets::CreateCharacterReply) => {
                let response = PacketServerCreateCharacterReply::try_from(packet)?;
                let message = match response.result {
                    CreateCharacterResult::Ok => Ok(CreateCharacterResponse { character_slot: 0 }),
                    CreateCharacterResult::NameAlreadyExists => {
                        Err(CreateCharacterError::AlreadyExists)
                    }
                    CreateCharacterResult::InvalidValue => Err(CreateCharacterError::InvalidValue),
                    CreateCharacterResult::NoMoreSlots => Err(CreateCharacterError::NoMoreSlots),
                    _ => Err(CreateCharacterError::Failed),
                };
                self.server_message_tx
                    .send(ServerMessage::CreateCharacter(message))
                    .ok();
            }
            // ServerPackets::DeleteCharacterReply -> ServerMessage::DeleteCharacter
            // ServerPackets::ReturnToCharacterSelect -> ServerMessage::ReturnToCharacterSelect
            _ => log::info!("Unhandled WorldClient packet {:?}", packet),
        }

        Ok(())
    }

    async fn handle_client_message(
        &self,
        connection: &mut Connection<'_>,
        message: ClientMessage,
    ) -> Result<(), anyhow::Error> {
        match message {
            ClientMessage::ConnectionRequest(ConnectionRequest {
                login_token,
                ref password_md5,
            }) => {
                connection
                    .write_packet(Packet::from(&PacketClientConnectRequest {
                        login_token,
                        password_md5,
                    }))
                    .await?
            }
            ClientMessage::GetCharacterList => {
                connection
                    .write_packet(Packet::from(&PacketClientCharacterList {}))
                    .await?
            }
            ClientMessage::SelectCharacter(message) => {
                connection
                    .write_packet(Packet::from(&PacketClientSelectCharacter {
                        slot: message.slot,
                        name: &message.name,
                    }))
                    .await?
            }
            ClientMessage::CreateCharacter(CreateCharacter {
                gender,
                birth_stone,
                hair,
                face,
                name,
            }) => {
                connection
                    .write_packet(Packet::from(&PacketClientCreateCharacter {
                        gender,
                        birth_stone,
                        hair,
                        face,
                        name: &name,
                        start_point: 0,
                    }))
                    .await?
            }
            // ClientMessage::DeleteCharacter -> ClientPackets::DeleteCharacter
            unimplemented => {
                log::info!(
                    "Unimplemented WorldClient ClientMessage {:?}",
                    unimplemented
                );
            }
        }
        Ok(())
    }
}

implement_protocol_client! { WorldClient }
