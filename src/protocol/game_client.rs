use num_traits::FromPrimitive;
use std::net::SocketAddr;
use thiserror::Error;
use tokio::net::TcpStream;

use rose_game_common::messages::{
    client::{ClientMessage, ConnectionRequest},
    server::{
        AnnounceChat, AttackEntity, CharacterData, CharacterDataItems, CharacterDataQuest,
        ConnectionRequestError, ConnectionResponse, DamageEntity, JoinZoneResponse, LocalChat,
        MoveEntity, PickupItemDropResult, RemoveEntities, ServerMessage, ShoutChat,
        SpawnEntityCharacter, SpawnEntityItemDrop, SpawnEntityMonster, SpawnEntityNpc,
        StopMoveEntity, Teleport, UpdateLevel, UpdateSpeed, UpdateXpStamina, Whisper,
    },
};
use rose_network_common::{Connection, Packet, PacketCodec};
use rose_network_irose::{
    game_client_packets::{
        PacketClientAttack, PacketClientChat, PacketClientConnectRequest, PacketClientJoinZone,
        PacketClientMove, PacketClientPickupItemDrop,
    },
    game_server_packets::{
        ConnectResult, PacketConnectionReply, PacketServerAnnounceChat, PacketServerAttackEntity,
        PacketServerCharacterInventory, PacketServerCharacterQuestData, PacketServerDamageEntity,
        PacketServerJoinZone, PacketServerLocalChat, PacketServerMoveEntity,
        PacketServerPickupItemDropResult, PacketServerRemoveEntities, PacketServerSelectCharacter,
        PacketServerShoutChat, PacketServerSpawnEntityCharacter, PacketServerSpawnEntityItemDrop,
        PacketServerSpawnEntityMonster, PacketServerSpawnEntityNpc, PacketServerStopMoveEntity,
        PacketServerTeleport, PacketServerUpdateLevel, PacketServerUpdateSpeed,
        PacketServerUpdateXpStamina, PacketServerWhisper, ServerPackets,
    },
    ClientPacketCodec, IROSE_112_TABLE,
};

#[derive(Debug, Error)]
pub enum GameClientError {
    #[error("client initiated disconnect")]
    ClientInitiatedDisconnect,
}

pub struct GameClient {
    server_address: SocketAddr,
    client_message_rx: tokio::sync::mpsc::UnboundedReceiver<ClientMessage>,
    #[allow(dead_code)]
    server_message_tx: crossbeam_channel::Sender<ServerMessage>,
    packet_codec: Box<dyn PacketCodec + Send + Sync>,
}

impl GameClient {
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
            Some(ServerPackets::SelectCharacter) => {
                let response = PacketServerSelectCharacter::try_from(&packet)?;
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
            Some(ServerPackets::CharacterInventory) => {
                let response = PacketServerCharacterInventory::try_from(&packet)?;
                self.server_message_tx
                    .send(ServerMessage::CharacterDataItems(Box::new(
                        CharacterDataItems {
                            inventory: response.inventory,
                            equipment: response.equipment,
                        },
                    )))
                    .ok();
            }
            Some(ServerPackets::QuestData) => {
                let response = PacketServerCharacterQuestData::try_from(&packet)?;
                self.server_message_tx
                    .send(ServerMessage::CharacterDataQuest(Box::new(
                        CharacterDataQuest {
                            quest_state: response.quest_state,
                        },
                    )))
                    .ok();
            }
            Some(ServerPackets::JoinZone) => {
                let response = PacketServerJoinZone::try_from(&packet)?;
                self.server_message_tx
                    .send(ServerMessage::JoinZone(JoinZoneResponse {
                        entity_id: response.entity_id,
                        experience_points: response.experience_points,
                        team: response.team,
                        health_points: response.health_points,
                        mana_points: response.mana_points,
                        world_ticks: response.world_ticks,
                    }))
                    .ok();
            }
            Some(ServerPackets::MoveEntity) | Some(ServerPackets::MoveEntityWithMoveMode) => {
                let response = PacketServerMoveEntity::try_from(&packet)?;
                self.server_message_tx
                    .send(ServerMessage::MoveEntity(MoveEntity {
                        entity_id: response.entity_id,
                        target_entity_id: response.target_entity_id,
                        distance: response.distance,
                        x: response.x,
                        y: response.y,
                        z: response.z,
                        move_mode: response.move_mode,
                    }))
                    .ok();
            }
            Some(ServerPackets::StopMoveEntity) => {
                let response = PacketServerStopMoveEntity::try_from(&packet)?;
                self.server_message_tx
                    .send(ServerMessage::StopMoveEntity(StopMoveEntity {
                        entity_id: response.entity_id,
                        x: response.x,
                        y: response.y,
                        z: response.z,
                    }))
                    .ok();
            }
            Some(ServerPackets::AttackEntity) => {
                let response = PacketServerAttackEntity::try_from(&packet)?;
                self.server_message_tx
                    .send(ServerMessage::AttackEntity(AttackEntity {
                        entity_id: response.entity_id,
                        target_entity_id: response.target_entity_id,
                        distance: response.distance,
                        x: response.x,
                        y: response.y,
                        z: response.z,
                    }))
                    .ok();
            }
            Some(ServerPackets::PickupItemDropResult) => {
                let message = PacketServerPickupItemDropResult::try_from(&packet)?;
                self.server_message_tx
                    .send(ServerMessage::PickupItemDropResult(PickupItemDropResult {
                        item_entity_id: message.item_entity_id,
                        result: message.result,
                    }))
                    .ok();
            }
            Some(ServerPackets::SpawnEntityCharacter) => {
                let message = PacketServerSpawnEntityCharacter::try_from(&packet)?;
                self.server_message_tx
                    .send(ServerMessage::SpawnEntityCharacter(Box::new(
                        SpawnEntityCharacter {
                            entity_id: message.entity_id,
                            position: message.position,
                            team: message.team,
                            health: message.health,
                            destination: message.destination,
                            command: message.command,
                            target_entity_id: message.target_entity_id,
                            move_mode: message.move_mode,
                            status_effects: message.status_effects,
                            character_info: message.character_info,
                            equipment: message.equipment,
                            level: message.level,
                            move_speed: message.move_speed,
                            passive_attack_speed: message.passive_attack_speed,
                            personal_store_info: message.personal_store_info,
                        },
                    )))
                    .ok();
            }
            Some(ServerPackets::SpawnEntityNpc) => {
                let message = PacketServerSpawnEntityNpc::try_from(&packet)?;
                self.server_message_tx
                    .send(ServerMessage::SpawnEntityNpc(SpawnEntityNpc {
                        entity_id: message.entity_id,
                        npc: message.npc,
                        direction: message.direction,
                        position: message.position,
                        team: message.team,
                        health: message.health,
                        destination: message.destination,
                        command: message.command,
                        target_entity_id: message.target_entity_id,
                        move_mode: message.move_mode,
                        status_effects: message.status_effects,
                    }))
                    .ok();
            }
            Some(ServerPackets::SpawnEntityMonster) => {
                let message = PacketServerSpawnEntityMonster::try_from(&packet)?;
                self.server_message_tx
                    .send(ServerMessage::SpawnEntityMonster(SpawnEntityMonster {
                        entity_id: message.entity_id,
                        npc: message.npc,
                        position: message.position,
                        team: message.team,
                        health: message.health,
                        destination: message.destination,
                        command: message.command,
                        target_entity_id: message.target_entity_id,
                        move_mode: message.move_mode,
                        status_effects: message.status_effects,
                    }))
                    .ok();
            }
            Some(ServerPackets::SpawnEntityItemDrop) => {
                let message = PacketServerSpawnEntityItemDrop::try_from(&packet)?;
                self.server_message_tx
                    .send(ServerMessage::SpawnEntityItemDrop(SpawnEntityItemDrop {
                        entity_id: message.entity_id,
                        position: message.position,
                        dropped_item: message.dropped_item,
                        remaining_time: message.remaining_time,
                        owner_entity_id: message.owner_entity_id,
                    }))
                    .ok();
            }
            Some(ServerPackets::DamageEntity) => {
                let message = PacketServerDamageEntity::try_from(&packet)?;
                self.server_message_tx
                    .send(ServerMessage::DamageEntity(DamageEntity {
                        attacker_entity_id: message.attacker_entity_id,
                        defender_entity_id: message.defender_entity_id,
                        damage: message.damage,
                        is_killed: message.is_killed,
                        from_skill: None,
                    }))
                    .ok();
            }
            Some(ServerPackets::RemoveEntities) => {
                let message = PacketServerRemoveEntities::try_from(&packet)?;
                self.server_message_tx
                    .send(ServerMessage::RemoveEntities(RemoveEntities {
                        entity_ids: message.entity_ids,
                    }))
                    .ok();
            }
            Some(ServerPackets::Teleport) => {
                let message = PacketServerTeleport::try_from(&packet)?;
                self.server_message_tx
                    .send(ServerMessage::Teleport(Teleport {
                        entity_id: message.entity_id,
                        zone_id: message.zone_id,
                        x: message.x,
                        y: message.y,
                        run_mode: message.run_mode,
                        ride_mode: message.ride_mode,
                    }))
                    .ok();
            }
            Some(ServerPackets::LocalChat) => {
                let message = PacketServerLocalChat::try_from(&packet)?;
                self.server_message_tx
                    .send(ServerMessage::LocalChat(LocalChat {
                        entity_id: message.entity_id,
                        text: message.text.to_string(),
                    }))
                    .ok();
            }
            Some(ServerPackets::ShoutChat) => {
                let message = PacketServerShoutChat::try_from(&packet)?;
                self.server_message_tx
                    .send(ServerMessage::ShoutChat(ShoutChat {
                        name: message.name.to_string(),
                        text: message.text.to_string(),
                    }))
                    .ok();
            }
            Some(ServerPackets::AnnounceChat) => {
                let message = PacketServerAnnounceChat::try_from(&packet)?;
                self.server_message_tx
                    .send(ServerMessage::AnnounceChat(AnnounceChat {
                        name: message.name.map(|x| x.to_string()),
                        text: message.text.to_string(),
                    }))
                    .ok();
            }
            Some(ServerPackets::Whisper) => {
                let message = PacketServerWhisper::try_from(&packet)?;
                self.server_message_tx
                    .send(ServerMessage::Whisper(Whisper {
                        from: message.from.to_string(),
                        text: message.text.to_string(),
                    }))
                    .ok();
            }
            Some(ServerPackets::UpdateLevel) => {
                let message = PacketServerUpdateLevel::try_from(&packet)?;
                self.server_message_tx
                    .send(ServerMessage::UpdateLevel(UpdateLevel {
                        entity_id: message.entity_id,
                        level: message.level,
                        experience_points: message.experience_points,
                        stat_points: message.stat_points,
                        skill_points: message.skill_points,
                    }))
                    .ok();
            }
            Some(ServerPackets::UpdateSpeed) => {
                let message = PacketServerUpdateSpeed::try_from(&packet)?;
                self.server_message_tx
                    .send(ServerMessage::UpdateSpeed(UpdateSpeed {
                        entity_id: message.entity_id,
                        run_speed: message.run_speed,
                        passive_attack_speed: message.passive_attack_speed,
                    }))
                    .ok();
            }
            Some(ServerPackets::UpdateXpStamina) => {
                let message = PacketServerUpdateXpStamina::try_from(&packet)?;
                self.server_message_tx
                    .send(ServerMessage::UpdateXpStamina(UpdateXpStamina {
                        xp: message.xp,
                        stamina: message.stamina,
                        source_entity_id: message.source_entity_id,
                    }))
                    .ok();
            }
            _ => println!("Unhandled game packet {:x}", packet.command),
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
            ClientMessage::Move(message) => {
                connection
                    .write_packet(Packet::from(&PacketClientMove {
                        target_entity_id: message.target_entity_id,
                        x: message.x,
                        y: message.y,
                        z: message.z,
                    }))
                    .await?
            }
            ClientMessage::Attack(message) => {
                connection
                    .write_packet(Packet::from(&PacketClientAttack {
                        target_entity_id: message.target_entity_id,
                    }))
                    .await?
            }
            ClientMessage::PickupItemDrop(target_entity_id) => {
                connection
                    .write_packet(Packet::from(&PacketClientPickupItemDrop {
                        target_entity_id,
                    }))
                    .await?
            }
            ClientMessage::Chat(ref text) => {
                connection
                    .write_packet(Packet::from(&PacketClientChat { text }))
                    .await?
            }
            unimplemented => {
                println!("Unimplemented GameClient ClientMessage {:?}", unimplemented);
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
                        return Err(GameClientError::ClientInitiatedDisconnect.into());
                    }
                }
            };
        }

        // Ok(())
    }
}
