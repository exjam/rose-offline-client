use async_trait::async_trait;
use num_traits::FromPrimitive;
use std::net::SocketAddr;
use tokio::net::TcpStream;

use rose_game_common::messages::{
    client::{ClientMessage, ConnectionRequest},
    server::{
        CharacterData, ConnectionRequestError, ConnectionResponse, JoinZoneResponse, ServerMessage,
    },
};
use rose_network_common::{Connection, Packet, PacketCodec};
use rose_network_narose667::{
    game_client_packets::{PacketClientConnectRequest, PacketClientJoinZone},
    game_server_packets::{
        ConnectResult, PacketConnectionReply, PacketServerJoinZone, PacketServerSelectCharacter,
        ServerPackets,
    },
    ClientPacketCodec,
};

use crate::protocol::{ProtocolClient, ProtocolClientError};

pub struct GameClient {
    server_address: SocketAddr,
    client_message_rx: tokio::sync::mpsc::UnboundedReceiver<ClientMessage>,
    server_message_tx: crossbeam_channel::Sender<ServerMessage>,
    packet_codec: Box<dyn PacketCodec + Send + Sync>,
}

impl GameClient {
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
            Some(ServerPackets::SelectCharacter) => {
                let response = PacketServerSelectCharacter::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::CharacterData(Box::new(CharacterData {
                        character_info: response.character_info,
                        position: response.position,
                        zone_id: response.zone_id,
                        basic_stats: response.basic_stats,
                        level: response.level,
                        equipment: response.equipment,
                        experience_points: response.experience_points,
                        skill_list: response.skill_list,
                        hotbar: response.hotbar,
                        health_points: response.health_points,
                        mana_points: response.mana_points,
                        stat_points: response.stat_points,
                        skill_points: response.skill_points,
                        union_membership: response.union_membership,
                        stamina: response.stamina,
                    })))
                    .ok();
            }
            Some(ServerPackets::JoinZone) => {
                let response = PacketServerJoinZone::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::JoinZone(JoinZoneResponse {
                        entity_id: response.entity_id,
                        experience_points: response.experience_points,
                        team: response.team,
                        health_points: response.health_points,
                        mana_points: response.mana_points,
                        world_ticks: response.world_ticks,
                        craft_rate: response.craft_rate,
                        world_price_rate: response.world_price_rate,
                        item_price_rate: response.item_price_rate,
                        town_price_rate: response.town_price_rate,
                    }))
                    .ok();
            }
            _ => log::info!("Unhandled game packet {:?}", packet),
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
            ClientMessage::JoinZoneRequest => {
                connection
                    .write_packet(Packet::from(&PacketClientJoinZone {
                        weight_rate: 0,
                        z: 0,
                    }))
                    .await?
            }
            unimplemented => {
                log::info!("Unimplemented GameClient ClientMessage {:?}", unimplemented);
            }
        }
        Ok(())
    }
}

implement_protocol_client! { GameClient }
