use async_trait::async_trait;
use num_traits::FromPrimitive;
use std::net::SocketAddr;
use tokio::net::TcpStream;

use rose_data::{QuestTriggerHash, SkillId};
use rose_game_common::{
    components::MoveMode,
    messages::{
        client::{ChangeEquipment, ClientMessage, ConnectionRequest, QuestDelete},
        server::{
            self, AnnounceChat, ApplySkillEffect, AttackEntity, CastSkillSelf,
            CastSkillTargetEntity, CastSkillTargetPosition, CharacterData, CharacterDataItems,
            CharacterDataQuest, ConnectionRequestError, ConnectionResponse, DamageEntity,
            JoinZoneResponse, LevelUpSkillResult, LocalChat, MoveEntity, PickupItemDropResult,
            QuestDeleteResult, QuestTriggerResult, RemoveEntities, ServerMessage, ShoutChat,
            SpawnEntityCharacter, SpawnEntityItemDrop, SpawnEntityMonster, SpawnEntityNpc,
            StopMoveEntity, Teleport, UpdateAbilityValue, UpdateBasicStat, UpdateEquipment,
            UpdateLevel, UpdateSpeed, UpdateStatusEffects, UpdateVehiclePart, UpdateXpStamina,
            UseEmote, UseInventoryItem, UseItem, Whisper,
        },
    },
};
use rose_network_common::{Connection, Packet, PacketCodec};
use rose_network_irose::{
    game_client_packets::{
        PacketClientAttack, PacketClientCastSkillSelf, PacketClientCastSkillTargetEntity,
        PacketClientCastSkillTargetPosition, PacketClientChangeAmmo, PacketClientChangeEquipment,
        PacketClientChangeVehiclePart, PacketClientChat, PacketClientConnectRequest,
        PacketClientDropItemFromInventory, PacketClientEmote, PacketClientIncreaseBasicStat,
        PacketClientJoinZone, PacketClientLevelUpSkill, PacketClientMove,
        PacketClientMoveCollision, PacketClientMoveToggle, PacketClientMoveToggleType,
        PacketClientNpcStoreTransaction, PacketClientPartyReply, PacketClientPartyRequest,
        PacketClientPartyUpdateRules, PacketClientPersonalStoreListItems,
        PacketClientPickupItemDrop, PacketClientQuestRequest, PacketClientQuestRequestType,
        PacketClientReviveRequest, PacketClientSetHotbarSlot, PacketClientUseItem,
        PacketClientWarpGateRequest,
    },
    game_server_packets::{
        ConnectResult, PacketConnectionReply, PacketServerAdjustPosition, PacketServerAnnounceChat,
        PacketServerApplySkillDamage, PacketServerApplySkillEffect, PacketServerAttackEntity,
        PacketServerCancelCastingSkill, PacketServerCastSkillSelf,
        PacketServerCastSkillTargetEntity, PacketServerCastSkillTargetPosition,
        PacketServerChangeNpcId, PacketServerCharacterInventory, PacketServerCharacterQuestData,
        PacketServerDamageEntity, PacketServerFinishCastingSkill, PacketServerJoinZone,
        PacketServerLearnSkillResult, PacketServerLevelUpSkillResult, PacketServerLocalChat,
        PacketServerMoveEntity, PacketServerMoveToggle, PacketServerMoveToggleType,
        PacketServerNpcStoreTransactionError, PacketServerPartyMemberRewardItem,
        PacketServerPartyMemberUpdateInfo, PacketServerPartyMembers, PacketServerPartyReply,
        PacketServerPartyRequest, PacketServerPartyUpdateRules, PacketServerPickupItemDropResult,
        PacketServerQuestResult, PacketServerQuestResultType, PacketServerRemoveEntities,
        PacketServerRewardItems, PacketServerRewardMoney, PacketServerRunNpcDeathTrigger,
        PacketServerSelectCharacter, PacketServerSetHotbarSlot, PacketServerShoutChat,
        PacketServerSpawnEntityCharacter, PacketServerSpawnEntityItemDrop,
        PacketServerSpawnEntityMonster, PacketServerSpawnEntityNpc, PacketServerStartCastingSkill,
        PacketServerStopMoveEntity, PacketServerTeleport, PacketServerUpdateAbilityValue,
        PacketServerUpdateAmmo, PacketServerUpdateBasicStat, PacketServerUpdateEquipment,
        PacketServerUpdateInventory, PacketServerUpdateLevel, PacketServerUpdateMoney,
        PacketServerUpdateSpeed, PacketServerUpdateStatusEffects, PacketServerUpdateVehiclePart,
        PacketServerUpdateXpStamina, PacketServerUseEmote, PacketServerUseItem,
        PacketServerWhisper, ServerPackets,
    },
    ClientPacketCodec, IROSE_112_TABLE,
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
            Some(ServerPackets::CharacterInventory) => {
                let response = PacketServerCharacterInventory::try_from(packet)?;
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
                let response = PacketServerCharacterQuestData::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::CharacterDataQuest(Box::new(
                        CharacterDataQuest {
                            quest_state: response.quest_state,
                        },
                    )))
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
            Some(ServerPackets::MoveEntity) | Some(ServerPackets::MoveEntityWithMoveMode) => {
                let response = PacketServerMoveEntity::try_from(packet)?;
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
                let response = PacketServerStopMoveEntity::try_from(packet)?;
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
                let response = PacketServerAttackEntity::try_from(packet)?;
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
                let message = PacketServerPickupItemDropResult::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::PickupItemDropResult(PickupItemDropResult {
                        item_entity_id: message.item_entity_id,
                        result: message.result,
                    }))
                    .ok();
            }
            Some(ServerPackets::SpawnEntityCharacter) => {
                let message = PacketServerSpawnEntityCharacter::try_from(packet)?;
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
                let message = PacketServerSpawnEntityNpc::try_from(packet)?;
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
                let message = PacketServerSpawnEntityMonster::try_from(packet)?;
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
                let message = PacketServerSpawnEntityItemDrop::try_from(packet)?;
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
                let message = PacketServerDamageEntity::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::DamageEntity(DamageEntity {
                        attacker_entity_id: message.attacker_entity_id,
                        defender_entity_id: message.defender_entity_id,
                        damage: message.damage,
                        is_killed: message.is_killed,
                        is_immediate: message.is_immediate,
                        from_skill: None,
                    }))
                    .ok();
            }
            Some(ServerPackets::RemoveEntities) => {
                let message = PacketServerRemoveEntities::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::RemoveEntities(RemoveEntities {
                        entity_ids: message.entity_ids,
                    }))
                    .ok();
            }
            Some(ServerPackets::Teleport) => {
                let message = PacketServerTeleport::try_from(packet)?;
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
                let message = PacketServerLocalChat::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::LocalChat(LocalChat {
                        entity_id: message.entity_id,
                        text: message.text.to_string(),
                    }))
                    .ok();
            }
            Some(ServerPackets::ShoutChat) => {
                let message = PacketServerShoutChat::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::ShoutChat(ShoutChat {
                        name: message.name.to_string(),
                        text: message.text.to_string(),
                    }))
                    .ok();
            }
            Some(ServerPackets::AnnounceChat) => {
                let message = PacketServerAnnounceChat::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::AnnounceChat(AnnounceChat {
                        name: message.name.map(|x| x.to_string()),
                        text: message.text.to_string(),
                    }))
                    .ok();
            }
            Some(ServerPackets::Whisper) => {
                let message = PacketServerWhisper::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::Whisper(Whisper {
                        from: message.from.to_string(),
                        text: message.text.to_string(),
                    }))
                    .ok();
            }
            Some(ServerPackets::UpdateAmmo) => {
                let message = PacketServerUpdateAmmo::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::UpdateAmmo(
                        message.entity_id,
                        message.ammo_index,
                        message.item,
                    ))
                    .ok();
            }
            Some(ServerPackets::UpdateEquipment) => {
                let message = PacketServerUpdateEquipment::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::UpdateEquipment(UpdateEquipment {
                        entity_id: message.entity_id,
                        equipment_index: message.equipment_index,
                        item: message.item,
                    }))
                    .ok();
            }
            Some(ServerPackets::UpdateInventory) | Some(ServerPackets::UpdateMoneyAndInventory) => {
                let message = PacketServerUpdateInventory::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::UpdateInventory(
                        message.items,
                        message.with_money,
                    ))
                    .ok();
            }
            Some(ServerPackets::UpdateMoney) => {
                let message = PacketServerUpdateMoney::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::UpdateMoney(message.money))
                    .ok();
            }
            Some(ServerPackets::UpdateVehiclePart) => {
                let message = PacketServerUpdateVehiclePart::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::UpdateVehiclePart(UpdateVehiclePart {
                        entity_id: message.entity_id,
                        vehicle_part_index: message.vehicle_part_index,
                        item: message.item,
                    }))
                    .ok();
            }
            Some(ServerPackets::UpdateBasicStat) => {
                let message = PacketServerUpdateBasicStat::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::UpdateBasicStat(UpdateBasicStat {
                        basic_stat_type: message.basic_stat_type,
                        value: message.value,
                    }))
                    .ok();
            }
            Some(ServerPackets::UpdateAbilityValueRewardAdd)
            | Some(ServerPackets::UpdateAbilityValueRewardSet) => {
                let message = PacketServerUpdateAbilityValue::try_from(packet)?;
                if message.is_add {
                    self.server_message_tx
                        .send(ServerMessage::UpdateAbilityValue(
                            UpdateAbilityValue::RewardAdd(message.ability_type, message.value),
                        ))
                        .ok();
                } else {
                    self.server_message_tx
                        .send(ServerMessage::UpdateAbilityValue(
                            UpdateAbilityValue::RewardSet(message.ability_type, message.value),
                        ))
                        .ok();
                }
            }
            Some(ServerPackets::UpdateLevel) => {
                let message = PacketServerUpdateLevel::try_from(packet)?;
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
                let message = PacketServerUpdateSpeed::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::UpdateSpeed(UpdateSpeed {
                        entity_id: message.entity_id,
                        run_speed: message.run_speed,
                        passive_attack_speed: message.passive_attack_speed,
                    }))
                    .ok();
            }
            Some(ServerPackets::UpdateStatusEffects) => {
                let message = PacketServerUpdateStatusEffects::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::UpdateStatusEffects(UpdateStatusEffects {
                        entity_id: message.entity_id,
                        status_effects: message.status_effects,
                        updated_hp: message.updated_hp,
                        updated_mp: message.updated_mp,
                    }))
                    .ok();
            }
            Some(ServerPackets::UpdateXpStamina) => {
                let message = PacketServerUpdateXpStamina::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::UpdateXpStamina(UpdateXpStamina {
                        xp: message.xp,
                        stamina: message.stamina,
                        source_entity_id: message.source_entity_id,
                    }))
                    .ok();
            }
            Some(ServerPackets::QuestResult) => {
                let message = PacketServerQuestResult::try_from(packet)?;
                match message.result {
                    PacketServerQuestResultType::DeleteSuccess => {
                        self.server_message_tx
                            .send(ServerMessage::QuestDeleteResult(QuestDeleteResult {
                                success: true,
                                slot: message.slot as usize,
                                quest_id: message.quest_id as usize,
                            }))
                            .ok();
                    }
                    PacketServerQuestResultType::DeleteFailed => {
                        self.server_message_tx
                            .send(ServerMessage::QuestDeleteResult(QuestDeleteResult {
                                success: false,
                                slot: message.slot as usize,
                                quest_id: message.quest_id as usize,
                            }))
                            .ok();
                    }
                    PacketServerQuestResultType::TriggerSuccess => {
                        self.server_message_tx
                            .send(ServerMessage::QuestTriggerResult(QuestTriggerResult {
                                success: true,
                                trigger_hash: QuestTriggerHash {
                                    hash: message.quest_id,
                                },
                            }))
                            .ok();
                    }
                    PacketServerQuestResultType::TriggerFailed => {
                        self.server_message_tx
                            .send(ServerMessage::QuestTriggerResult(QuestTriggerResult {
                                success: false,
                                trigger_hash: QuestTriggerHash {
                                    hash: message.quest_id,
                                },
                            }))
                            .ok();
                    }
                    _ => {}
                }
            }
            Some(ServerPackets::RunNpcDeathTrigger) => {
                let message = PacketServerRunNpcDeathTrigger::try_from(packet)?;

                self.server_message_tx
                    .send(ServerMessage::RunNpcDeathTrigger(message.npc_id))
                    .ok();
            }
            Some(ServerPackets::RewardMoney) => {
                let message = PacketServerRewardMoney::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::RewardMoney(message.money))
                    .ok();
            }
            Some(ServerPackets::RewardItems) => {
                let message = PacketServerRewardItems::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::RewardItems(message.items))
                    .ok();
            }
            Some(ServerPackets::SetHotbarSlot) => {
                let message = PacketServerSetHotbarSlot::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::SetHotbarSlot(
                        message.slot_index,
                        message.slot,
                    ))
                    .ok();
            }
            Some(ServerPackets::LearnSkillResult) => {
                let message = PacketServerLearnSkillResult::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::LearnSkillResult(message.result))
                    .ok();
            }
            Some(ServerPackets::LevelUpSkillResult) => {
                let message = PacketServerLevelUpSkillResult::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::LevelUpSkillResult(LevelUpSkillResult {
                        result: message.result,
                        updated_skill_points: message.updated_skill_points,
                    }))
                    .ok();
            }
            Some(ServerPackets::UseEmote) => {
                let message = PacketServerUseEmote::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::UseEmote(UseEmote {
                        entity_id: message.entity_id,
                        motion_id: message.motion_id,
                        is_stop: message.is_stop,
                    }))
                    .ok();
            }
            Some(ServerPackets::UseItem) => {
                let message = PacketServerUseItem::try_from(packet)?;
                if let Some(inventory_slot) = message.inventory_slot {
                    self.server_message_tx
                        .send(ServerMessage::UseInventoryItem(UseInventoryItem {
                            entity_id: message.entity_id,
                            item: message.item,
                            inventory_slot,
                        }))
                        .ok();
                } else {
                    self.server_message_tx
                        .send(ServerMessage::UseItem(UseItem {
                            entity_id: message.entity_id,
                            item: message.item,
                        }))
                        .ok();
                }
            }
            Some(ServerPackets::ChangeNpcId) => {
                let message = PacketServerChangeNpcId::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::ChangeNpcId(
                        message.client_entity_id,
                        message.npc_id,
                    ))
                    .ok();
            }
            Some(ServerPackets::CastSkillSelf) => {
                let message = PacketServerCastSkillSelf::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::CastSkillSelf(CastSkillSelf {
                        entity_id: message.entity_id,
                        skill_id: message.skill_id,
                        cast_motion_id: message.cast_motion_id,
                    }))
                    .ok();
            }
            Some(ServerPackets::CastSkillTargetEntity) => {
                let message = PacketServerCastSkillTargetEntity::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::CastSkillTargetEntity(
                        CastSkillTargetEntity {
                            entity_id: message.entity_id,
                            skill_id: message.skill_id,
                            cast_motion_id: message.cast_motion_id,
                            target_entity_id: message.target_entity_id,
                            target_distance: message.target_distance,
                            target_position: message.target_position,
                        },
                    ))
                    .ok();
            }
            Some(ServerPackets::CastSkillTargetPosition) => {
                let message = PacketServerCastSkillTargetPosition::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::CastSkillTargetPosition(
                        CastSkillTargetPosition {
                            entity_id: message.entity_id,
                            skill_id: message.skill_id,
                            cast_motion_id: message.cast_motion_id,
                            target_position: message.target_position,
                        },
                    ))
                    .ok();
            }
            Some(ServerPackets::StartCastingSkill) => {
                let message = PacketServerStartCastingSkill::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::StartCastingSkill(message.entity_id))
                    .ok();
            }
            Some(ServerPackets::CancelCastingSkill) => {
                let message = PacketServerCancelCastingSkill::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::CancelCastingSkill(
                        message.entity_id,
                        message.reason,
                    ))
                    .ok();
            }
            Some(ServerPackets::FinishCastingSkill) => {
                let message = PacketServerFinishCastingSkill::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::FinishCastingSkill(
                        message.entity_id,
                        message.skill_id,
                    ))
                    .ok();
            }
            Some(ServerPackets::ApplySkillEffect) => {
                let message = PacketServerApplySkillEffect::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::ApplySkillEffect(ApplySkillEffect {
                        entity_id: message.entity_id,
                        caster_entity_id: message.caster_entity_id,
                        caster_intelligence: message.caster_intelligence,
                        skill_id: message.skill_id,
                        effect_success: message.effect_success,
                    }))
                    .ok();
            }
            Some(ServerPackets::ApplySkillDamage) => {
                let message = PacketServerApplySkillDamage::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::DamageEntity(DamageEntity {
                        attacker_entity_id: message.caster_entity_id,
                        defender_entity_id: message.entity_id,
                        damage: message.damage,
                        is_killed: message.is_killed,
                        is_immediate: message.is_immediate,
                        from_skill: Some((message.skill_id, message.caster_intelligence)),
                    }))
                    .ok();
            }
            Some(ServerPackets::MoveToggle) => {
                let message = PacketServerMoveToggle::try_from(packet)?;
                match message.move_toggle_type {
                    PacketServerMoveToggleType::Walk => {
                        self.server_message_tx
                            .send(ServerMessage::MoveToggle(server::MoveToggle {
                                entity_id: message.entity_id,
                                move_mode: MoveMode::Walk,
                                run_speed: message.run_speed,
                            }))
                            .ok();
                    }
                    PacketServerMoveToggleType::Run => {
                        self.server_message_tx
                            .send(ServerMessage::MoveToggle(server::MoveToggle {
                                entity_id: message.entity_id,
                                move_mode: MoveMode::Run,
                                run_speed: message.run_speed,
                            }))
                            .ok();
                    }
                    PacketServerMoveToggleType::Drive => {
                        self.server_message_tx
                            .send(ServerMessage::MoveToggle(server::MoveToggle {
                                entity_id: message.entity_id,
                                move_mode: MoveMode::Drive,
                                run_speed: message.run_speed,
                            }))
                            .ok();
                    }
                    PacketServerMoveToggleType::Sit => {
                        self.server_message_tx
                            .send(ServerMessage::SitToggle(message.entity_id))
                            .ok();
                    }
                }
            }
            Some(ServerPackets::NpcStoreTransactionError) => {
                let message = PacketServerNpcStoreTransactionError::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::NpcStoreTransactionError(message.error))
                    .ok();
            }
            Some(ServerPackets::PartyRequest) => {
                let message = match PacketServerPartyRequest::try_from(packet)? {
                    PacketServerPartyRequest::Create(client_entity_id) => {
                        ServerMessage::PartyCreate(client_entity_id)
                    }
                    PacketServerPartyRequest::Invite(client_entity_id) => {
                        ServerMessage::PartyInvite(client_entity_id)
                    }
                };
                self.server_message_tx.send(message).ok();
            }
            Some(ServerPackets::PartyReply) => {
                let message = match PacketServerPartyReply::try_from(packet)? {
                    PacketServerPartyReply::AcceptCreate(client_entity_id) => {
                        ServerMessage::PartyAcceptCreate(client_entity_id)
                    }
                    PacketServerPartyReply::AcceptInvite(client_entity_id) => {
                        ServerMessage::PartyAcceptInvite(client_entity_id)
                    }
                    PacketServerPartyReply::RejectInvite(reason, client_entity_id) => {
                        ServerMessage::PartyRejectInvite(reason, client_entity_id)
                    }
                    PacketServerPartyReply::Delete => ServerMessage::PartyDelete,
                    PacketServerPartyReply::ChangeOwner(client_entity_id) => {
                        ServerMessage::PartyChangeOwner(client_entity_id)
                    }
                    PacketServerPartyReply::MemberKicked(character_unique_id) => {
                        ServerMessage::PartyMemberKicked(character_unique_id)
                    }
                    PacketServerPartyReply::MemberDisconnect(character_unique_id) => {
                        ServerMessage::PartyMemberDisconnect(character_unique_id)
                    }
                };
                self.server_message_tx.send(message).ok();
            }
            Some(ServerPackets::PartyMembers) => {
                let message = match PacketServerPartyMembers::try_from(packet)? {
                    PacketServerPartyMembers::Leave(party_member_leave) => {
                        ServerMessage::PartyMemberLeave(party_member_leave)
                    }
                    PacketServerPartyMembers::List(party_member_list) => {
                        ServerMessage::PartyMemberList(party_member_list)
                    }
                };
                self.server_message_tx.send(message).ok();
            }
            Some(ServerPackets::PartyMemberUpdateInfo) => {
                let message = PacketServerPartyMemberUpdateInfo::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::PartyMemberUpdateInfo(message.member_info))
                    .ok();
            }
            Some(ServerPackets::PartyMemberRewardItem) => {
                let message = PacketServerPartyMemberRewardItem::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::PartyMemberRewardItem {
                        client_entity_id: message.entity_id,
                        item: message.item,
                    })
                    .ok();
            }
            Some(ServerPackets::PartyUpdateRules) => {
                let message = PacketServerPartyUpdateRules::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::PartyUpdateRules(
                        message.item_sharing,
                        message.xp_sharing,
                    ))
                    .ok();
            }
            Some(ServerPackets::AdjustPosition) => {
                let message = PacketServerAdjustPosition::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::AdjustPosition(
                        message.client_entity_id,
                        message.position,
                    ))
                    .ok();
            }
            _ => log::info!("Unhandled GameClient packet {:?}", packet),
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
                ref password,
            }) => {
                connection
                    .write_packet(Packet::from(&PacketClientConnectRequest {
                        login_token,
                        password_md5: &password.to_md5(),
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
            ClientMessage::ChangeAmmo(ammo_index, item_slot) => {
                connection
                    .write_packet(Packet::from(&PacketClientChangeAmmo {
                        ammo_index,
                        item_slot,
                    }))
                    .await?
            }
            ClientMessage::ChangeEquipment(ChangeEquipment {
                equipment_index,
                item_slot,
            }) => {
                connection
                    .write_packet(Packet::from(&PacketClientChangeEquipment {
                        equipment_index,
                        item_slot,
                    }))
                    .await?
            }
            ClientMessage::ChangeVehiclePart(vehicle_part_index, item_slot) => {
                connection
                    .write_packet(Packet::from(&PacketClientChangeVehiclePart {
                        vehicle_part_index,
                        item_slot,
                    }))
                    .await?
            }
            ClientMessage::QuestDelete(QuestDelete { slot, quest_id }) => {
                connection
                    .write_packet(Packet::from(&PacketClientQuestRequest {
                        request_type: PacketClientQuestRequestType::DeleteQuest,
                        quest_slot: slot as u8,
                        quest_id: quest_id as u32,
                    }))
                    .await?
            }
            ClientMessage::QuestTrigger(quest_id) => {
                connection
                    .write_packet(Packet::from(&PacketClientQuestRequest {
                        request_type: PacketClientQuestRequestType::DoTrigger,
                        quest_slot: 0,
                        quest_id: quest_id.hash,
                    }))
                    .await?
            }
            ClientMessage::SetHotbarSlot(message) => {
                connection
                    .write_packet(Packet::from(&PacketClientSetHotbarSlot {
                        slot_index: message.slot_index,
                        slot: message.slot,
                    }))
                    .await?
            }
            ClientMessage::IncreaseBasicStat(basic_stat_type) => {
                connection
                    .write_packet(Packet::from(&PacketClientIncreaseBasicStat {
                        basic_stat_type,
                    }))
                    .await?
            }
            ClientMessage::ReviveRequest(revive_request_type) => {
                connection
                    .write_packet(Packet::from(&PacketClientReviveRequest {
                        revive_request_type,
                    }))
                    .await?
            }
            ClientMessage::PersonalStoreListItems(target_entity_id) => {
                connection
                    .write_packet(Packet::from(&PacketClientPersonalStoreListItems {
                        target_entity_id,
                    }))
                    .await?
            }
            ClientMessage::DropItem(item_slot, quantity) => {
                connection
                    .write_packet(Packet::from(&PacketClientDropItemFromInventory::Item(
                        item_slot,
                        quantity as u32,
                    )))
                    .await?
            }
            ClientMessage::DropMoney(quantity) => {
                connection
                    .write_packet(Packet::from(&PacketClientDropItemFromInventory::Money(
                        quantity as u32,
                    )))
                    .await?
            }
            ClientMessage::UseItem(item_slot, target_entity_id) => {
                connection
                    .write_packet(Packet::from(&PacketClientUseItem {
                        item_slot,
                        target_entity_id,
                    }))
                    .await?
            }
            ClientMessage::WarpGateRequest(warp_gate_id) => {
                connection
                    .write_packet(Packet::from(&PacketClientWarpGateRequest { warp_gate_id }))
                    .await?
            }
            ClientMessage::LevelUpSkill(skill_slot) => {
                connection
                    .write_packet(Packet::from(&PacketClientLevelUpSkill {
                        skill_slot,
                        next_skill_idx: SkillId::new(1).unwrap(), // TODO: next_skill_idx
                    }))
                    .await?
            }
            ClientMessage::UseEmote(motion_id, is_stop) => {
                connection
                    .write_packet(Packet::from(&PacketClientEmote { motion_id, is_stop }))
                    .await?
            }
            ClientMessage::CastSkillSelf(skill_slot) => {
                connection
                    .write_packet(Packet::from(&PacketClientCastSkillSelf { skill_slot }))
                    .await?
            }
            ClientMessage::CastSkillTargetEntity(skill_slot, target_entity_id) => {
                connection
                    .write_packet(Packet::from(&PacketClientCastSkillTargetEntity {
                        skill_slot,
                        target_entity_id,
                    }))
                    .await?
            }
            ClientMessage::CastSkillTargetPosition(skill_slot, position) => {
                connection
                    .write_packet(Packet::from(&PacketClientCastSkillTargetPosition {
                        skill_slot,
                        position,
                    }))
                    .await?
            }
            ClientMessage::RunToggle => {
                connection
                    .write_packet(Packet::from(&PacketClientMoveToggle {
                        toggle_type: PacketClientMoveToggleType::Run,
                    }))
                    .await?
            }
            ClientMessage::SitToggle => {
                connection
                    .write_packet(Packet::from(&PacketClientMoveToggle {
                        toggle_type: PacketClientMoveToggleType::Sit,
                    }))
                    .await?
            }
            ClientMessage::DriveToggle => {
                connection
                    .write_packet(Packet::from(&PacketClientMoveToggle {
                        toggle_type: PacketClientMoveToggleType::Drive,
                    }))
                    .await?
            }
            ClientMessage::NpcStoreTransaction(message) => {
                connection
                    .write_packet(Packet::from(&PacketClientNpcStoreTransaction {
                        npc_entity_id: message.npc_entity_id,
                        buy_items: message.buy_items,
                        sell_items: message.sell_items,
                    }))
                    .await?
            }
            ClientMessage::PartyCreate(client_entity_id) => {
                connection
                    .write_packet(Packet::from(&PacketClientPartyRequest::Create(
                        client_entity_id,
                    )))
                    .await?
            }
            ClientMessage::PartyInvite(client_entity_id) => {
                connection
                    .write_packet(Packet::from(&PacketClientPartyRequest::Invite(
                        client_entity_id,
                    )))
                    .await?
            }
            ClientMessage::PartyLeave => {
                connection
                    .write_packet(Packet::from(&PacketClientPartyRequest::Leave))
                    .await?
            }
            ClientMessage::PartyChangeOwner(client_entity_id) => {
                connection
                    .write_packet(Packet::from(&PacketClientPartyRequest::ChangeOwner(
                        client_entity_id,
                    )))
                    .await?
            }
            ClientMessage::PartyKick(character_unique_id) => {
                connection
                    .write_packet(Packet::from(&PacketClientPartyRequest::Kick(
                        character_unique_id,
                    )))
                    .await?
            }
            ClientMessage::PartyAcceptCreateInvite(client_entity_id) => {
                connection
                    .write_packet(Packet::from(&PacketClientPartyReply::AcceptCreate(
                        client_entity_id,
                    )))
                    .await?
            }
            ClientMessage::PartyAcceptJoinInvite(client_entity_id) => {
                connection
                    .write_packet(Packet::from(&PacketClientPartyReply::AcceptJoin(
                        client_entity_id,
                    )))
                    .await?
            }
            ClientMessage::PartyRejectInvite(reason, client_entity_id) => {
                connection
                    .write_packet(Packet::from(&PacketClientPartyReply::Reject(
                        reason,
                        client_entity_id,
                    )))
                    .await?
            }
            ClientMessage::PartyUpdateRules(item_sharing, xp_sharing) => {
                connection
                    .write_packet(Packet::from(&PacketClientPartyUpdateRules {
                        item_sharing,
                        xp_sharing,
                    }))
                    .await?
            }
            ClientMessage::MoveCollision(position) => {
                connection
                    .write_packet(Packet::from(&PacketClientMoveCollision { position }))
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
