use async_trait::async_trait;
use num_traits::FromPrimitive;
use std::net::SocketAddr;
use tokio::net::TcpStream;

use rose_game_common::{
    components::CharacterDeleteTime,
    messages::{
        client::{ClientMessage, ConnectionRequest, CreateCharacter, DeleteCharacter},
        server::{
            ConnectionRequestError, ConnectionResponse, CreateCharacterError,
            CreateCharacterResponse, DeleteCharacterError, DeleteCharacterResponse,
            JoinServerResponse, ServerMessage,
        },
    },
};
use rose_network_common::{Connection, Packet, PacketCodec};
use rose_network_irose::{
    world_client_packets::{
        PacketClientCharacterList, PacketClientConnectRequest, PacketClientCreateCharacter,
        PacketClientDeleteCharacter, PacketClientSelectCharacter,
    },
    world_server_packets::{
        ConnectResult, CreateCharacterResult, PacketConnectionReply, PacketServerCharacterList,
        PacketServerCreateCharacterReply, PacketServerDeleteCharacterReply, PacketServerMoveServer,
        ServerPackets,
    },
    ClientPacketCodec, IROSE_112_TABLE,
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
        packet_codec_seed: u32,
        client_message_rx: tokio::sync::mpsc::UnboundedReceiver<ClientMessage>,
        server_message_tx: crossbeam_channel::Sender<ServerMessage>,
    ) -> Self {
        Self {
            server_address,
            client_message_rx,
            server_message_tx,
            packet_codec: Box::new(ClientPacketCodec::init(&IROSE_112_TABLE, packet_codec_seed)),
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
            Some(ServerPackets::DeleteCharacterReply) => {
                let response = PacketServerDeleteCharacterReply::try_from(packet)?;
                let result = match response.seconds_until_delete {
                    Some(0) => Ok(DeleteCharacterResponse {
                        name: response.name.into(),
                        delete_time: None,
                    }),
                    Some(delete_time) => Ok(DeleteCharacterResponse {
                        name: response.name.into(),
                        delete_time: Some(CharacterDeleteTime::from_seconds_remaining(delete_time)),
                    }),
                    None => Err(DeleteCharacterError::Failed("Failed".into())),
                };
                self.server_message_tx
                    .send(ServerMessage::DeleteCharacter(result))
                    .ok();
            }
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
                hair,
                face,
                name,
                start_point,
                birth_stone,
                ..
            }) => {
                connection
                    .write_packet(Packet::from(&PacketClientCreateCharacter {
                        gender,
                        birth_stone: birth_stone as u8,
                        hair: hair as u8,
                        face: face as u8,
                        name: &name,
                        start_point: start_point as u16,
                    }))
                    .await?
            }
            ClientMessage::DeleteCharacter(DeleteCharacter {
                slot,
                name,
                is_delete,
            }) => {
                connection
                    .write_packet(Packet::from(&PacketClientDeleteCharacter {
                        slot,
                        name: &name,
                        is_delete,
                    }))
                    .await?
            }
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
