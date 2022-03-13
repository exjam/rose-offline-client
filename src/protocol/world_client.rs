use num_traits::FromPrimitive;
use std::net::SocketAddr;
use thiserror::Error;
use tokio::net::TcpStream;

use rose_game_common::messages::{
    client::{ClientMessage, ConnectionRequest},
    server::{ConnectionRequestError, ConnectionResponse, ServerMessage},
};
use rose_network_common::{Connection, Packet, PacketCodec};
use rose_network_irose::{
    world_client_packets::{PacketClientCharacterList, PacketClientConnectRequest},
    world_server_packets::{ConnectResult, PacketConnectionReply, ServerPackets, PacketServerCharacterList},
    ClientPacketCodec, IROSE_112_TABLE,
};

#[derive(Debug, Error)]
pub enum WorldClientError {
    #[error("client initiated disconnect")]
    ClientInitiatedDisconnect,
}

pub struct WorldClient {
    server_address: SocketAddr,
    client_message_rx: tokio::sync::mpsc::UnboundedReceiver<ClientMessage>,
    #[allow(dead_code)]
    server_message_tx: crossbeam_channel::Sender<ServerMessage>,
    packet_codec: Box<dyn PacketCodec + Send + Sync>,
}

impl WorldClient {
    // TODO: Pass irose into this
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

    async fn handle_packet(&self, packet: Packet) -> Result<(), anyhow::Error> {
        match FromPrimitive::from_u16(packet.command) {
            Some(ServerPackets::ConnectReply) => {
                let response = PacketConnectionReply::try_from(&packet)?;
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
                    .send(ServerMessage::CharacterList(PacketServerCharacterList::try_from(&packet)?.characters))
                    .ok();
            }
            // ServerPackets::CreateCharacterReply -> ServerMessage::CreateCharacter
            // ServerPackets::DeleteCharacterReply -> ServerMessage::DeleteCharacter
            // ServerPackets::MoveServer -> ServerMessage::SelectCharacter
            // ServerPackets::ReturnToCharacterSelect -> ServerMessage::ReturnToCharacterSelect
            _ => println!("Unhandled world packet {:x}", packet.command),
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
            // ClientMessage::CreateCharacter -> ClientPackets::CreateCharacter
            // ClientMessage::DeleteCharacter -> ClientPackets::DeleteCharacter
            // ClientMessage::SelectCharacter -> ClientPackets::SelectCharacter
            unimplemented => {
                println!(
                    "Unimplemented WorldClient ClientMessage {:?}",
                    unimplemented
                );
            }
        }
        Ok(())
    }

    pub async fn run_connection(&mut self) -> Result<(), anyhow::Error> {
        let socket = TcpStream::connect(&self.server_address).await?;
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
                        return Err(WorldClientError::ClientInitiatedDisconnect.into());
                    }
                }
            };
        }

        // Ok(())
    }
}
