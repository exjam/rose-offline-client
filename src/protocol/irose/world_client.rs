use async_trait::async_trait;
use num_traits::FromPrimitive;
use std::net::SocketAddr;
use tokio::net::TcpStream;

use rose_game_common::{
    components::CharacterDeleteTime,
    messages::{
        client::ClientMessage,
        server::{ConnectionRequestError, CreateCharacterError, ServerMessage},
    },
};
use rose_network_common::{Connection, Packet, PacketCodec};
use rose_network_irose::{
    world_client_packets::{
        PacketClientCharacterList, PacketClientClanCommand, PacketClientConnectRequest,
        PacketClientCreateCharacter, PacketClientDeleteCharacter, PacketClientSelectCharacter,
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
                    ConnectResult::Ok => ServerMessage::ConnectionRequestSuccess {
                        packet_sequence_id: response.packet_sequence_id,
                    },
                    _ => ServerMessage::ConnectionRequestError {
                        error: ConnectionRequestError::Failed,
                    },
                };
                self.server_message_tx.send(message).ok();
            }
            Some(ServerPackets::CharacterListReply) => {
                self.server_message_tx
                    .send(ServerMessage::CharacterList {
                        character_list: PacketServerCharacterList::try_from(packet)?.characters,
                    })
                    .ok();
            }
            Some(ServerPackets::MoveServer) => {
                let response = PacketServerMoveServer::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::SelectCharacterSuccess {
                        login_token: response.login_token,
                        packet_codec_seed: response.packet_codec_seed,
                        ip: response.ip.to_string(),
                        port: response.port,
                    })
                    .ok();
            }
            Some(ServerPackets::CreateCharacterReply) => {
                let response = PacketServerCreateCharacterReply::try_from(packet)?;
                let message = match response.result {
                    CreateCharacterResult::Ok => {
                        ServerMessage::CreateCharacterSuccess { character_slot: 0 }
                    }
                    CreateCharacterResult::NameAlreadyExists => {
                        ServerMessage::CreateCharacterError {
                            error: CreateCharacterError::AlreadyExists,
                        }
                    }

                    CreateCharacterResult::InvalidValue => ServerMessage::CreateCharacterError {
                        error: CreateCharacterError::InvalidValue,
                    },
                    CreateCharacterResult::NoMoreSlots => ServerMessage::CreateCharacterError {
                        error: CreateCharacterError::NoMoreSlots,
                    },
                    _ => ServerMessage::CreateCharacterError {
                        error: CreateCharacterError::Failed,
                    },
                };
                self.server_message_tx.send(message).ok();
            }
            Some(ServerPackets::DeleteCharacterReply) => {
                let response = PacketServerDeleteCharacterReply::try_from(packet)?;
                let message = match response.seconds_until_delete {
                    Some(0) => ServerMessage::DeleteCharacterCancel {
                        name: response.name.into(),
                    },
                    Some(delete_time) => ServerMessage::DeleteCharacterStart {
                        name: response.name.into(),
                        delete_time: CharacterDeleteTime::from_seconds_remaining(delete_time),
                    },
                    None => ServerMessage::DeleteCharacterError {
                        name: response.name.into(),
                    },
                };
                self.server_message_tx.send(message).ok();
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
            ClientMessage::ConnectionRequest {
                login_token,
                ref password,
            } => {
                connection
                    .write_packet(Packet::from(&PacketClientConnectRequest {
                        login_token,
                        password_md5: &password.to_md5(),
                    }))
                    .await?
            }
            ClientMessage::GetCharacterList => {
                connection
                    .write_packet(Packet::from(&PacketClientCharacterList {}))
                    .await?
            }
            ClientMessage::SelectCharacter { slot, ref name } => {
                connection
                    .write_packet(Packet::from(&PacketClientSelectCharacter { slot, name }))
                    .await?
            }
            ClientMessage::CreateCharacter {
                gender,
                hair,
                face,
                name,
                start_point,
                birth_stone,
                ..
            } => {
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
            ClientMessage::DeleteCharacter {
                slot,
                name,
                is_delete,
            } => {
                connection
                    .write_packet(Packet::from(&PacketClientDeleteCharacter {
                        slot,
                        name: &name,
                        is_delete,
                    }))
                    .await?
            }
            ClientMessage::ClanGetMemberList => {
                connection
                    .write_packet(Packet::from(&PacketClientClanCommand::GetMemberList))
                    .await?
            }
            ClientMessage::ClanUpdateCharacterInfo { job, level } => {
                connection
                    .write_packet(Packet::from(&PacketClientClanCommand::UpdateLevelAndJob {
                        level,
                        job,
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
