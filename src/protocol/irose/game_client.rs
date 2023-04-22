use async_trait::async_trait;
use num_traits::FromPrimitive;
use std::net::SocketAddr;
use tokio::net::TcpStream;

use rose_data::{QuestTriggerHash, SkillId};
use rose_game_common::{
    components::MoveMode,
    messages::{
        client::ClientMessage,
        server::{
            CharacterData, CharacterDataItems, ConnectionRequestError, ServerMessage,
            SpawnEntityCharacter,
        },
    },
};
use rose_network_common::{Connection, Packet, PacketCodec};
use rose_network_irose::{
    game_client_packets::{
        PacketClientAttack, PacketClientBankMoveItem, PacketClientBankOpen,
        PacketClientCastSkillSelf, PacketClientCastSkillTargetEntity,
        PacketClientCastSkillTargetPosition, PacketClientChangeAmmo, PacketClientChangeEquipment,
        PacketClientChangeVehiclePart, PacketClientChat, PacketClientClanCommand,
        PacketClientConnectRequest, PacketClientCraftItem, PacketClientDropItemFromInventory,
        PacketClientEmote, PacketClientIncreaseBasicStat, PacketClientJoinZone,
        PacketClientLevelUpSkill, PacketClientMove, PacketClientMoveCollision,
        PacketClientMoveToggle, PacketClientMoveToggleType, PacketClientNpcStoreTransaction,
        PacketClientPartyReply, PacketClientPartyRequest, PacketClientPartyUpdateRules,
        PacketClientPersonalStoreBuyItem, PacketClientPersonalStoreListItems,
        PacketClientPickupItemDrop, PacketClientQuestRequest, PacketClientQuestRequestType,
        PacketClientRepairItemUsingItem, PacketClientRepairItemUsingNpc, PacketClientReviveRequest,
        PacketClientSetHotbarSlot, PacketClientSetReviveZone, PacketClientUseItem,
        PacketClientWarpGateRequest,
    },
    game_server_packets::{
        ConnectResult, PacketConnectionReply, PacketServerAdjustPosition, PacketServerAnnounceChat,
        PacketServerApplySkillDamage, PacketServerApplySkillEffect, PacketServerAttackEntity,
        PacketServerBankOpen, PacketServerBankTransaction, PacketServerCancelCastingSkill,
        PacketServerCastSkillSelf, PacketServerCastSkillTargetEntity,
        PacketServerCastSkillTargetPosition, PacketServerChangeNpcId,
        PacketServerCharacterInventory, PacketServerCharacterQuestData, PacketServerClanCommand,
        PacketServerClosePersonalStore, PacketServerCraftItem, PacketServerDamageEntity,
        PacketServerFinishCastingSkill, PacketServerJoinZone, PacketServerLearnSkillResult,
        PacketServerLevelUpSkillResult, PacketServerLocalChat, PacketServerLogoutResult,
        PacketServerMoveEntity, PacketServerMoveToggle, PacketServerMoveToggleType,
        PacketServerNpcStoreTransactionError, PacketServerOpenPersonalStore,
        PacketServerPartyMemberRewardItem, PacketServerPartyMemberUpdateInfo,
        PacketServerPartyMembers, PacketServerPartyReply, PacketServerPartyRequest,
        PacketServerPartyUpdateRules, PacketServerPersonalStoreItemList,
        PacketServerPersonalStoreTransactionResult,
        PacketServerPersonalStoreTransactionUpdateMoneyAndInventory,
        PacketServerPickupItemDropResult, PacketServerQuestResult, PacketServerQuestResultType,
        PacketServerRemoveEntities, PacketServerRepairedItemUsingNpc, PacketServerRewardItems,
        PacketServerRewardMoney, PacketServerRunNpcDeathTrigger, PacketServerSelectCharacter,
        PacketServerSetHotbarSlot, PacketServerShoutChat, PacketServerSpawnEntityCharacter,
        PacketServerSpawnEntityItemDrop, PacketServerSpawnEntityMonster,
        PacketServerSpawnEntityNpc, PacketServerStartCastingSkill, PacketServerStopMoveEntity,
        PacketServerTeleport, PacketServerUpdateAbilityValue, PacketServerUpdateAmmo,
        PacketServerUpdateBasicStat, PacketServerUpdateEquipment, PacketServerUpdateInventory,
        PacketServerUpdateItemLife, PacketServerUpdateLevel, PacketServerUpdateMoney,
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
                    ConnectResult::Ok => ServerMessage::ConnectionRequestSuccess {
                        packet_sequence_id: response.packet_sequence_id,
                    },
                    ConnectResult::InvalidPassword => ServerMessage::ConnectionRequestError {
                        error: ConnectionRequestError::InvalidPassword,
                    },
                    _ => ServerMessage::ConnectionRequestError {
                        error: ConnectionRequestError::Failed,
                    },
                };
                self.server_message_tx.send(message).ok();
            }
            Some(ServerPackets::SelectCharacter) => {
                let response = PacketServerSelectCharacter::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::CharacterData {
                        data: Box::new(CharacterData {
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
                        }),
                    })
                    .ok();
            }
            Some(ServerPackets::CharacterInventory) => {
                let response = PacketServerCharacterInventory::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::CharacterDataItems {
                        data: Box::new(CharacterDataItems {
                            inventory: response.inventory,
                            equipment: response.equipment,
                        }),
                    })
                    .ok();
            }
            Some(ServerPackets::QuestData) => {
                let response = PacketServerCharacterQuestData::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::CharacterDataQuest {
                        quest_state: Box::new(response.quest_state),
                    })
                    .ok();
            }
            Some(ServerPackets::JoinZone) => {
                let response = PacketServerJoinZone::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::JoinZone {
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
                    })
                    .ok();
            }
            Some(ServerPackets::MoveEntity) | Some(ServerPackets::MoveEntityWithMoveMode) => {
                let response = PacketServerMoveEntity::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::MoveEntity {
                        entity_id: response.entity_id,
                        target_entity_id: response.target_entity_id,
                        distance: response.distance,
                        x: response.x,
                        y: response.y,
                        z: response.z,
                        move_mode: response.move_mode,
                    })
                    .ok();
            }
            Some(ServerPackets::StopMoveEntity) => {
                let response = PacketServerStopMoveEntity::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::StopMoveEntity {
                        entity_id: response.entity_id,
                        x: response.x,
                        y: response.y,
                        z: response.z,
                    })
                    .ok();
            }
            Some(ServerPackets::AttackEntity) => {
                let response = PacketServerAttackEntity::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::AttackEntity {
                        entity_id: response.entity_id,
                        target_entity_id: response.target_entity_id,
                        distance: response.distance,
                        x: response.x,
                        y: response.y,
                        z: response.z,
                    })
                    .ok();
            }
            Some(ServerPackets::PickupItemDropResult) => {
                let message = match PacketServerPickupItemDropResult::try_from(packet)? {
                    PacketServerPickupItemDropResult::Item {
                        drop_entity_id,
                        item_slot,
                        item,
                    } => ServerMessage::PickupDropItem {
                        drop_entity_id,
                        item_slot,
                        item,
                    },
                    PacketServerPickupItemDropResult::Money {
                        drop_entity_id,
                        money,
                    } => ServerMessage::PickupDropMoney {
                        drop_entity_id,
                        money,
                    },
                    PacketServerPickupItemDropResult::Error {
                        drop_entity_id,
                        error,
                    } => ServerMessage::PickupDropError {
                        drop_entity_id,
                        error,
                    },
                };
                self.server_message_tx.send(message).ok();
            }
            Some(ServerPackets::SpawnEntityCharacter) => {
                let message = PacketServerSpawnEntityCharacter::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::SpawnEntityCharacter {
                        data: Box::new(SpawnEntityCharacter {
                            entity_id: message.entity_id,
                            position: message.position,
                            team: message.team,
                            health: message.health,
                            spawn_command_state: message.spawn_command_state,
                            move_mode: message.move_mode,
                            status_effects: message.status_effects,
                            character_info: message.character_info,
                            equipment: message.equipment,
                            level: message.level,
                            move_speed: message.move_speed,
                            passive_attack_speed: message.passive_attack_speed,
                            personal_store_info: message.personal_store_info,
                            clan_membership: message.clan_membership,
                        }),
                    })
                    .ok();
            }
            Some(ServerPackets::SpawnEntityNpc) => {
                let message = PacketServerSpawnEntityNpc::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::SpawnEntityNpc {
                        entity_id: message.entity_id,
                        npc: message.npc,
                        direction: message.direction,
                        position: message.position,
                        team: message.team,
                        health: message.health,
                        spawn_command_state: message.spawn_command_state,
                        move_mode: message.move_mode,
                        status_effects: message.status_effects,
                    })
                    .ok();
            }
            Some(ServerPackets::SpawnEntityMonster) => {
                let message = PacketServerSpawnEntityMonster::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::SpawnEntityMonster {
                        entity_id: message.entity_id,
                        npc: message.npc,
                        position: message.position,
                        team: message.team,
                        health: message.health,
                        spawn_command_state: message.spawn_command_state,
                        move_mode: message.move_mode,
                        status_effects: message.status_effects,
                    })
                    .ok();
            }
            Some(ServerPackets::SpawnEntityItemDrop) => {
                let message = PacketServerSpawnEntityItemDrop::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::SpawnEntityItemDrop {
                        entity_id: message.entity_id,
                        position: message.position,
                        dropped_item: message.dropped_item,
                        remaining_time: message.remaining_time,
                        owner_entity_id: message.owner_entity_id,
                    })
                    .ok();
            }
            Some(ServerPackets::DamageEntity) => {
                let message = PacketServerDamageEntity::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::DamageEntity {
                        attacker_entity_id: message.attacker_entity_id,
                        defender_entity_id: message.defender_entity_id,
                        damage: message.damage,
                        is_killed: message.is_killed,
                        is_immediate: message.is_immediate,
                        from_skill: None,
                    })
                    .ok();
            }
            Some(ServerPackets::RemoveEntities) => {
                let message = PacketServerRemoveEntities::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::RemoveEntities {
                        entity_ids: message.entity_ids,
                    })
                    .ok();
            }
            Some(ServerPackets::Teleport) => {
                let message = PacketServerTeleport::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::Teleport {
                        entity_id: message.entity_id,
                        zone_id: message.zone_id,
                        x: message.x,
                        y: message.y,
                        run_mode: message.run_mode,
                        ride_mode: message.ride_mode,
                    })
                    .ok();
            }
            Some(ServerPackets::LocalChat) => {
                let message = PacketServerLocalChat::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::LocalChat {
                        entity_id: message.entity_id,
                        text: message.text.to_string(),
                    })
                    .ok();
            }
            Some(ServerPackets::ShoutChat) => {
                let message = PacketServerShoutChat::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::ShoutChat {
                        name: message.name.to_string(),
                        text: message.text.to_string(),
                    })
                    .ok();
            }
            Some(ServerPackets::AnnounceChat) => {
                let message = PacketServerAnnounceChat::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::AnnounceChat {
                        name: message.name.map(|x| x.to_string()),
                        text: message.text.to_string(),
                    })
                    .ok();
            }
            Some(ServerPackets::Whisper) => {
                let message = PacketServerWhisper::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::Whisper {
                        from: message.from.to_string(),
                        text: message.text.to_string(),
                    })
                    .ok();
            }
            Some(ServerPackets::UpdateAmmo) => {
                let PacketServerUpdateAmmo {
                    entity_id,
                    ammo_index,
                    item,
                } = packet.try_into()?;
                self.server_message_tx
                    .send(ServerMessage::UpdateAmmo {
                        entity_id,
                        ammo_index,
                        item,
                    })
                    .ok();
            }
            Some(ServerPackets::UpdateEquipment) => {
                let PacketServerUpdateEquipment {
                    entity_id,
                    equipment_index,
                    item,
                    ..
                } = packet.try_into()?;
                self.server_message_tx
                    .send(ServerMessage::UpdateEquipment {
                        entity_id,
                        equipment_index,
                        item,
                    })
                    .ok();
            }
            Some(ServerPackets::UpdateInventory) | Some(ServerPackets::UpdateMoneyAndInventory) => {
                let PacketServerUpdateInventory {
                    items,
                    with_money: money,
                } = packet.try_into()?;
                self.server_message_tx
                    .send(ServerMessage::UpdateInventory { items, money })
                    .ok();
            }
            Some(ServerPackets::UpdateMoney) => {
                let message = PacketServerUpdateMoney::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::UpdateMoney {
                        money: message.money,
                    })
                    .ok();
            }
            Some(ServerPackets::UpdateVehiclePart) => {
                let PacketServerUpdateVehiclePart {
                    entity_id,
                    vehicle_part_index,
                    item,
                    ..
                } = packet.try_into()?;
                self.server_message_tx
                    .send(ServerMessage::UpdateVehiclePart {
                        entity_id,
                        vehicle_part_index,
                        item,
                    })
                    .ok();
            }
            Some(ServerPackets::UpdateItemLife) => {
                let message = PacketServerUpdateItemLife::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::UpdateItemLife {
                        item_slot: message.item_slot,
                        life: message.life,
                    })
                    .ok();
            }
            Some(ServerPackets::UpdateBasicStat) => {
                let message = PacketServerUpdateBasicStat::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::UpdateBasicStat {
                        basic_stat_type: message.basic_stat_type,
                        value: message.value,
                    })
                    .ok();
            }
            Some(ServerPackets::UpdateAbilityValueRewardAdd)
            | Some(ServerPackets::UpdateAbilityValueRewardSet) => {
                let message = PacketServerUpdateAbilityValue::try_from(packet)?;
                if message.is_add {
                    self.server_message_tx
                        .send(ServerMessage::UpdateAbilityValueAdd {
                            ability_type: message.ability_type,
                            value: message.value,
                        })
                        .ok();
                } else {
                    self.server_message_tx
                        .send(ServerMessage::UpdateAbilityValueSet {
                            ability_type: message.ability_type,
                            value: message.value,
                        })
                        .ok();
                }
            }
            Some(ServerPackets::UpdateLevel) => {
                let message = PacketServerUpdateLevel::try_from(packet)?;
                if let Some((level, experience_points, stat_points, skill_points)) =
                    message.update_values
                {
                    self.server_message_tx
                        .send(ServerMessage::UpdateLevel {
                            entity_id: message.entity_id,
                            level,
                            experience_points,
                            stat_points,
                            skill_points,
                        })
                        .ok();
                } else {
                    self.server_message_tx
                        .send(ServerMessage::LevelUpEntity {
                            entity_id: message.entity_id,
                        })
                        .ok();
                }
            }
            Some(ServerPackets::UpdateSpeed) => {
                let message = PacketServerUpdateSpeed::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::UpdateSpeed {
                        entity_id: message.entity_id,
                        run_speed: message.run_speed,
                        passive_attack_speed: message.passive_attack_speed,
                    })
                    .ok();
            }
            Some(ServerPackets::UpdateStatusEffects) => {
                let message = PacketServerUpdateStatusEffects::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::UpdateStatusEffects {
                        entity_id: message.entity_id,
                        status_effects: message.status_effects,
                        updated_values: message.updated_values,
                    })
                    .ok();
            }
            Some(ServerPackets::UpdateXpStamina) => {
                let message = PacketServerUpdateXpStamina::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::UpdateXpStamina {
                        xp: message.xp,
                        stamina: message.stamina,
                        source_entity_id: message.source_entity_id,
                    })
                    .ok();
            }
            Some(ServerPackets::QuestResult) => {
                let message = PacketServerQuestResult::try_from(packet)?;
                match message.result {
                    PacketServerQuestResultType::DeleteSuccess => {
                        self.server_message_tx
                            .send(ServerMessage::QuestDeleteResult {
                                success: true,
                                slot: message.slot as usize,
                                quest_id: message.quest_id as usize,
                            })
                            .ok();
                    }
                    PacketServerQuestResultType::DeleteFailed => {
                        self.server_message_tx
                            .send(ServerMessage::QuestDeleteResult {
                                success: false,
                                slot: message.slot as usize,
                                quest_id: message.quest_id as usize,
                            })
                            .ok();
                    }
                    PacketServerQuestResultType::TriggerSuccess => {
                        self.server_message_tx
                            .send(ServerMessage::QuestTriggerResult {
                                success: true,
                                trigger_hash: QuestTriggerHash {
                                    hash: message.quest_id,
                                },
                            })
                            .ok();
                    }
                    PacketServerQuestResultType::TriggerFailed => {
                        self.server_message_tx
                            .send(ServerMessage::QuestTriggerResult {
                                success: false,
                                trigger_hash: QuestTriggerHash {
                                    hash: message.quest_id,
                                },
                            })
                            .ok();
                    }
                    _ => {}
                }
            }
            Some(ServerPackets::RunNpcDeathTrigger) => {
                let message = PacketServerRunNpcDeathTrigger::try_from(packet)?;

                self.server_message_tx
                    .send(ServerMessage::RunNpcDeathTrigger {
                        npc_id: message.npc_id,
                    })
                    .ok();
            }
            Some(ServerPackets::RewardMoney) => {
                let message = PacketServerRewardMoney::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::RewardMoney {
                        money: message.money,
                    })
                    .ok();
            }
            Some(ServerPackets::RewardItems) => {
                let message = PacketServerRewardItems::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::RewardItems {
                        items: message.items,
                    })
                    .ok();
            }
            Some(ServerPackets::SetHotbarSlot) => {
                let message = PacketServerSetHotbarSlot::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::SetHotbarSlot {
                        slot_index: message.slot_index,
                        slot: message.slot,
                    })
                    .ok();
            }
            Some(ServerPackets::LearnSkillResult) => {
                let message = match PacketServerLearnSkillResult::try_from(packet)? {
                    PacketServerLearnSkillResult::Success {
                        skill_slot,
                        skill_id,
                        updated_skill_points,
                    } => ServerMessage::LearnSkillSuccess {
                        skill_slot,
                        skill_id,
                        updated_skill_points,
                    },
                    PacketServerLearnSkillResult::Error { error } => {
                        ServerMessage::LearnSkillError { error }
                    }
                };
                self.server_message_tx.send(message).ok();
            }
            Some(ServerPackets::LevelUpSkillResult) => {
                let message = match PacketServerLevelUpSkillResult::try_from(packet)? {
                    PacketServerLevelUpSkillResult::Success {
                        skill_slot,
                        skill_id,
                        skill_points,
                    } => ServerMessage::LevelUpSkillSuccess {
                        skill_slot,
                        skill_id,
                        skill_points,
                    },
                    PacketServerLevelUpSkillResult::Error {
                        error,
                        skill_points,
                    } => ServerMessage::LevelUpSkillError {
                        error,
                        skill_points,
                    },
                };
                self.server_message_tx.send(message).ok();
            }
            Some(ServerPackets::UseEmote) => {
                let message = PacketServerUseEmote::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::UseEmote {
                        entity_id: message.entity_id,
                        motion_id: message.motion_id,
                        is_stop: message.is_stop,
                    })
                    .ok();
            }
            Some(ServerPackets::UseItem) => {
                let message = PacketServerUseItem::try_from(packet)?;
                if let Some(inventory_slot) = message.inventory_slot {
                    self.server_message_tx
                        .send(ServerMessage::UseInventoryItem {
                            entity_id: message.entity_id,
                            item: message.item,
                            inventory_slot,
                        })
                        .ok();
                } else {
                    self.server_message_tx
                        .send(ServerMessage::UseItem {
                            entity_id: message.entity_id,
                            item: message.item,
                        })
                        .ok();
                }
            }
            Some(ServerPackets::ChangeNpcId) => {
                let message = PacketServerChangeNpcId::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::ChangeNpcId {
                        entity_id: message.entity_id,
                        npc_id: message.npc_id,
                    })
                    .ok();
            }
            Some(ServerPackets::CastSkillSelf) => {
                let message = PacketServerCastSkillSelf::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::CastSkillSelf {
                        entity_id: message.entity_id,
                        skill_id: message.skill_id,
                        cast_motion_id: message.cast_motion_id,
                    })
                    .ok();
            }
            Some(ServerPackets::CastSkillTargetEntity) => {
                let message = PacketServerCastSkillTargetEntity::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::CastSkillTargetEntity {
                        entity_id: message.entity_id,
                        skill_id: message.skill_id,
                        cast_motion_id: message.cast_motion_id,
                        target_entity_id: message.target_entity_id,
                        target_distance: message.target_distance,
                        target_position: message.target_position,
                    })
                    .ok();
            }
            Some(ServerPackets::CastSkillTargetPosition) => {
                let message = PacketServerCastSkillTargetPosition::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::CastSkillTargetPosition {
                        entity_id: message.entity_id,
                        skill_id: message.skill_id,
                        cast_motion_id: message.cast_motion_id,
                        target_position: message.target_position,
                    })
                    .ok();
            }
            Some(ServerPackets::StartCastingSkill) => {
                let message = PacketServerStartCastingSkill::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::StartCastingSkill {
                        entity_id: message.entity_id,
                    })
                    .ok();
            }
            Some(ServerPackets::CancelCastingSkill) => {
                let message = PacketServerCancelCastingSkill::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::CancelCastingSkill {
                        entity_id: message.entity_id,
                        reason: message.reason,
                    })
                    .ok();
            }
            Some(ServerPackets::FinishCastingSkill) => {
                let message = PacketServerFinishCastingSkill::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::FinishCastingSkill {
                        entity_id: message.entity_id,
                        skill_id: message.skill_id,
                    })
                    .ok();
            }
            Some(ServerPackets::ApplySkillEffect) => {
                let message = PacketServerApplySkillEffect::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::ApplySkillEffect {
                        entity_id: message.entity_id,
                        caster_entity_id: message.caster_entity_id,
                        caster_intelligence: message.caster_intelligence,
                        skill_id: message.skill_id,
                        effect_success: message.effect_success,
                    })
                    .ok();
            }
            Some(ServerPackets::ApplySkillDamage) => {
                let message = PacketServerApplySkillDamage::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::DamageEntity {
                        attacker_entity_id: message.caster_entity_id,
                        defender_entity_id: message.entity_id,
                        damage: message.damage,
                        is_killed: message.is_killed,
                        is_immediate: message.is_immediate,
                        from_skill: Some((message.skill_id, message.caster_intelligence)),
                    })
                    .ok();
            }
            Some(ServerPackets::MoveToggle) => {
                let message = PacketServerMoveToggle::try_from(packet)?;
                match message.move_toggle_type {
                    PacketServerMoveToggleType::Walk => {
                        self.server_message_tx
                            .send(ServerMessage::MoveToggle {
                                entity_id: message.entity_id,
                                move_mode: MoveMode::Walk,
                                run_speed: message.run_speed,
                            })
                            .ok();
                    }
                    PacketServerMoveToggleType::Run => {
                        self.server_message_tx
                            .send(ServerMessage::MoveToggle {
                                entity_id: message.entity_id,
                                move_mode: MoveMode::Run,
                                run_speed: message.run_speed,
                            })
                            .ok();
                    }
                    PacketServerMoveToggleType::Drive => {
                        self.server_message_tx
                            .send(ServerMessage::MoveToggle {
                                entity_id: message.entity_id,
                                move_mode: MoveMode::Drive,
                                run_speed: message.run_speed,
                            })
                            .ok();
                    }
                    PacketServerMoveToggleType::Sit => {
                        self.server_message_tx
                            .send(ServerMessage::SitToggle {
                                entity_id: message.entity_id,
                            })
                            .ok();
                    }
                }
            }
            Some(ServerPackets::NpcStoreTransactionError) => {
                let message = PacketServerNpcStoreTransactionError::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::NpcStoreTransactionError {
                        error: message.error,
                    })
                    .ok();
            }
            Some(ServerPackets::PartyRequest) => {
                let message = match PacketServerPartyRequest::try_from(packet)? {
                    PacketServerPartyRequest::Create(entity_id) => {
                        ServerMessage::PartyCreate { entity_id }
                    }
                    PacketServerPartyRequest::Invite(entity_id) => {
                        ServerMessage::PartyInvite { entity_id }
                    }
                };
                self.server_message_tx.send(message).ok();
            }
            Some(ServerPackets::PartyReply) => {
                let message = match PacketServerPartyReply::try_from(packet)? {
                    PacketServerPartyReply::AcceptCreate(entity_id) => {
                        ServerMessage::PartyAcceptCreate { entity_id }
                    }
                    PacketServerPartyReply::AcceptInvite(entity_id) => {
                        ServerMessage::PartyAcceptInvite { entity_id }
                    }
                    PacketServerPartyReply::RejectInvite(reason, entity_id) => {
                        ServerMessage::PartyRejectInvite { reason, entity_id }
                    }
                    PacketServerPartyReply::Delete => ServerMessage::PartyDelete,
                    PacketServerPartyReply::ChangeOwner(entity_id) => {
                        ServerMessage::PartyChangeOwner { entity_id }
                    }
                    PacketServerPartyReply::MemberKicked(character_id) => {
                        ServerMessage::PartyMemberKicked { character_id }
                    }
                    PacketServerPartyReply::MemberDisconnect(character_id) => {
                        ServerMessage::PartyMemberDisconnect { character_id }
                    }
                };
                self.server_message_tx.send(message).ok();
            }
            Some(ServerPackets::PartyMembers) => {
                let message = match PacketServerPartyMembers::try_from(packet)? {
                    PacketServerPartyMembers::Leave {
                        leaver_character_id,
                        owner_character_id,
                    } => ServerMessage::PartyMemberLeave {
                        leaver_character_id,
                        owner_character_id,
                    },
                    PacketServerPartyMembers::List {
                        item_sharing,
                        xp_sharing,
                        owner_character_id,
                        members,
                    } => ServerMessage::PartyMemberList {
                        item_sharing,
                        xp_sharing,
                        owner_character_id,
                        members,
                    },
                };
                self.server_message_tx.send(message).ok();
            }
            Some(ServerPackets::PartyMemberUpdateInfo) => {
                let message = PacketServerPartyMemberUpdateInfo::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::PartyMemberUpdateInfo {
                        member_info: message.member_info,
                    })
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
                    .send(ServerMessage::PartyUpdateRules {
                        item_sharing: message.item_sharing,
                        xp_sharing: message.xp_sharing,
                    })
                    .ok();
            }
            Some(ServerPackets::AdjustPosition) => {
                let message = PacketServerAdjustPosition::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::AdjustPosition {
                        entity_id: message.entity_id,
                        position: message.position,
                    })
                    .ok();
            }
            Some(ServerPackets::PersonalStoreItemList) => {
                let message = PacketServerPersonalStoreItemList::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::PersonalStoreItemList {
                        sell_items: message.sell_items,
                        buy_items: message.buy_items,
                    })
                    .ok();
            }
            Some(ServerPackets::PersonalStoreTransactionResult) => {
                let message = PacketServerPersonalStoreTransactionResult::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::PersonalStoreTransaction {
                        status: message.status,
                        store_entity_id: message.store_entity_id,
                        update_store: message.update_store_items,
                    })
                    .ok();
            }
            Some(ServerPackets::PersonalStoreTransactionUpdateMoneyAndInventory) => {
                let message =
                    PacketServerPersonalStoreTransactionUpdateMoneyAndInventory::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::PersonalStoreTransactionUpdateInventory {
                        items: message.items,
                        money: message.money,
                    })
                    .ok();
            }
            Some(ServerPackets::BankOpen) => match PacketServerBankOpen::try_from(packet)? {
                PacketServerBankOpen::Open => {
                    self.server_message_tx.send(ServerMessage::BankOpen).ok();
                }
                PacketServerBankOpen::SetItems { items } => {
                    self.server_message_tx
                        .send(ServerMessage::BankSetItems { items })
                        .ok();
                }
                PacketServerBankOpen::UpdateItems { items } => {
                    self.server_message_tx
                        .send(ServerMessage::BankUpdateItems { items })
                        .ok();
                }
            },
            Some(ServerPackets::BankTransaction) => {
                let packet = PacketServerBankTransaction::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::BankTransaction {
                        inventory_item_slot: packet.inventory_item_slot,
                        inventory_item: packet.inventory_item,
                        inventory_money: packet.inventory_money,
                        bank_slot: packet.bank_slot,
                        bank_item: packet.bank_item,
                    })
                    .ok();
            }
            Some(ServerPackets::LogoutResult) => {
                let packet = PacketServerLogoutResult::try_from(packet)?;
                match packet.result {
                    Ok(_) => {
                        self.server_message_tx
                            .send(ServerMessage::LogoutSuccess)
                            .ok();
                    }
                    Err(wait_duration) => {
                        self.server_message_tx
                            .send(ServerMessage::LogoutFailed { wait_duration })
                            .ok();
                    }
                }
            }
            Some(ServerPackets::OpenPersonalStore) => {
                let packet = PacketServerOpenPersonalStore::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::OpenPersonalStore {
                        entity_id: packet.entity_id,
                        skin: packet.skin,
                        title: packet.title.into(),
                    })
                    .ok();
            }
            Some(ServerPackets::ClosePersonalStore) => {
                let packet = PacketServerClosePersonalStore::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::ClosePersonalStore {
                        entity_id: packet.entity_id,
                    })
                    .ok();
            }
            Some(ServerPackets::CraftItem) => {
                let packet = PacketServerCraftItem::try_from(packet)?;
                match packet {
                    PacketServerCraftItem::InsertGemFailed { error } => {
                        self.server_message_tx
                            .send(ServerMessage::CraftInsertGemError { error })
                            .ok();
                    }
                    PacketServerCraftItem::InsertGemSuccess { items } => {
                        self.server_message_tx
                            .send(ServerMessage::CraftInsertGem {
                                update_items: items,
                            })
                            .ok();
                    }
                }
            }
            Some(ServerPackets::RepairedItemUsingNpc) => {
                let packet = PacketServerRepairedItemUsingNpc::try_from(packet)?;
                self.server_message_tx
                    .send(ServerMessage::RepairedItemUsingNpc {
                        item_slot: packet.item_slot,
                        item: packet.item,
                        updated_money: packet.updated_money,
                    })
                    .ok();
            }
            Some(ServerPackets::ClanCommand) => {
                let packet = PacketServerClanCommand::try_from(packet)?;
                match packet {
                    PacketServerClanCommand::ClanInfo {
                        id,
                        name,
                        description,
                        mark,
                        level,
                        points,
                        money,
                        position,
                        contribution,
                        skills,
                    } => {
                        self.server_message_tx
                            .send(ServerMessage::ClanInfo {
                                id,
                                mark,
                                level,
                                points,
                                money,
                                name,
                                description,
                                position,
                                contribution,
                                skills,
                            })
                            .ok();
                    }
                    PacketServerClanCommand::ClanUpdateInfo {
                        id,
                        mark,
                        level,
                        points,
                        money,
                        skills,
                    } => {
                        self.server_message_tx
                            .send(ServerMessage::ClanUpdateInfo {
                                id,
                                mark,
                                level,
                                points,
                                money,
                                skills,
                            })
                            .ok();
                    }
                    PacketServerClanCommand::CharacterUpdateClan {
                        client_entity_id,
                        id,
                        name,
                        mark,
                        level,
                        position,
                    } => {
                        self.server_message_tx
                            .send(ServerMessage::CharacterUpdateClan {
                                client_entity_id,
                                id,
                                name,
                                mark,
                                level,
                                position,
                            })
                            .ok();
                    }
                    PacketServerClanCommand::ClanMemberConnected { name, channel_id } => {
                        self.server_message_tx
                            .send(ServerMessage::ClanMemberConnected { name, channel_id })
                            .ok();
                    }
                    PacketServerClanCommand::ClanMemberDisconnected { name } => {
                        self.server_message_tx
                            .send(ServerMessage::ClanMemberDisconnected { name })
                            .ok();
                    }
                    PacketServerClanCommand::ClanCreateError { error } => {
                        self.server_message_tx
                            .send(ServerMessage::ClanCreateError { error })
                            .ok();
                    }
                    PacketServerClanCommand::ClanMemberList { members } => {
                        self.server_message_tx
                            .send(ServerMessage::ClanMemberList { members })
                            .ok();
                    }
                }
            }
            Some(ServerPackets::RepairedItemUsingItem) => {
                log::info!(
                    "Unimplemented ServerPackets::RepairedItemUsingItem {:?}",
                    packet
                );
            }
            None => log::info!("Unhandled GameClient packet {:?}", packet),
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
            ClientMessage::JoinZoneRequest => {
                connection
                    .write_packet(Packet::from(&PacketClientJoinZone {
                        weight_rate: 0,
                        z: 0,
                    }))
                    .await?
            }
            ClientMessage::Move {
                target_entity_id,
                x,
                y,
                z,
            } => {
                connection
                    .write_packet(Packet::from(&PacketClientMove {
                        target_entity_id,
                        x,
                        y,
                        z,
                    }))
                    .await?
            }
            ClientMessage::Attack { target_entity_id } => {
                connection
                    .write_packet(Packet::from(&PacketClientAttack { target_entity_id }))
                    .await?
            }
            ClientMessage::PickupItemDrop { target_entity_id } => {
                connection
                    .write_packet(Packet::from(&PacketClientPickupItemDrop {
                        target_entity_id,
                    }))
                    .await?
            }
            ClientMessage::Chat { ref text } => {
                connection
                    .write_packet(Packet::from(&PacketClientChat { text }))
                    .await?
            }
            ClientMessage::ChangeAmmo {
                ammo_index,
                item_slot,
            } => {
                connection
                    .write_packet(Packet::from(&PacketClientChangeAmmo {
                        ammo_index,
                        item_slot,
                    }))
                    .await?
            }
            ClientMessage::ChangeEquipment {
                equipment_index,
                item_slot,
            } => {
                connection
                    .write_packet(Packet::from(&PacketClientChangeEquipment {
                        equipment_index,
                        item_slot,
                    }))
                    .await?
            }
            ClientMessage::ChangeVehiclePart {
                vehicle_part_index,
                item_slot,
            } => {
                connection
                    .write_packet(Packet::from(&PacketClientChangeVehiclePart {
                        vehicle_part_index,
                        item_slot,
                    }))
                    .await?
            }
            ClientMessage::QuestDelete { slot, quest_id } => {
                connection
                    .write_packet(Packet::from(&PacketClientQuestRequest {
                        request_type: PacketClientQuestRequestType::DeleteQuest,
                        quest_slot: slot as u8,
                        quest_id: quest_id as u32,
                    }))
                    .await?
            }
            ClientMessage::QuestTrigger { trigger } => {
                connection
                    .write_packet(Packet::from(&PacketClientQuestRequest {
                        request_type: PacketClientQuestRequestType::DoTrigger,
                        quest_slot: 0,
                        quest_id: trigger.hash,
                    }))
                    .await?
            }
            ClientMessage::SetHotbarSlot { slot_index, slot } => {
                connection
                    .write_packet(Packet::from(&PacketClientSetHotbarSlot {
                        slot_index,
                        slot,
                    }))
                    .await?
            }
            ClientMessage::IncreaseBasicStat { basic_stat_type } => {
                connection
                    .write_packet(Packet::from(&PacketClientIncreaseBasicStat {
                        basic_stat_type,
                    }))
                    .await?
            }
            ClientMessage::ReviveCurrentZone => {
                connection
                    .write_packet(Packet::from(&PacketClientReviveRequest::CurrentZone))
                    .await?
            }
            ClientMessage::ReviveSaveZone => {
                connection
                    .write_packet(Packet::from(&PacketClientReviveRequest::SaveZone))
                    .await?
            }
            ClientMessage::PersonalStoreListItems { store_entity_id } => {
                connection
                    .write_packet(Packet::from(&PacketClientPersonalStoreListItems {
                        target_entity_id: store_entity_id,
                    }))
                    .await?
            }
            ClientMessage::DropItem {
                item_slot,
                quantity,
            } => {
                connection
                    .write_packet(Packet::from(&PacketClientDropItemFromInventory::Item(
                        item_slot,
                        quantity as u32,
                    )))
                    .await?
            }
            ClientMessage::DropMoney { quantity } => {
                connection
                    .write_packet(Packet::from(&PacketClientDropItemFromInventory::Money(
                        quantity as u32,
                    )))
                    .await?
            }
            ClientMessage::UseItem {
                item_slot,
                target_entity_id,
            } => {
                connection
                    .write_packet(Packet::from(&PacketClientUseItem {
                        item_slot,
                        target_entity_id,
                    }))
                    .await?
            }
            ClientMessage::WarpGateRequest { warp_gate_id } => {
                connection
                    .write_packet(Packet::from(&PacketClientWarpGateRequest { warp_gate_id }))
                    .await?
            }
            ClientMessage::LevelUpSkill { skill_slot } => {
                connection
                    .write_packet(Packet::from(&PacketClientLevelUpSkill {
                        skill_slot,
                        next_skill_idx: SkillId::new(1).unwrap(), // TODO: next_skill_idx
                    }))
                    .await?
            }
            ClientMessage::UseEmote { motion_id, is_stop } => {
                connection
                    .write_packet(Packet::from(&PacketClientEmote { motion_id, is_stop }))
                    .await?
            }
            ClientMessage::CastSkillSelf { skill_slot } => {
                connection
                    .write_packet(Packet::from(&PacketClientCastSkillSelf { skill_slot }))
                    .await?
            }
            ClientMessage::CastSkillTargetEntity {
                skill_slot,
                target_entity_id,
            } => {
                connection
                    .write_packet(Packet::from(&PacketClientCastSkillTargetEntity {
                        skill_slot,
                        target_entity_id,
                    }))
                    .await?
            }
            ClientMessage::CastSkillTargetPosition {
                skill_slot,
                position,
            } => {
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
            ClientMessage::NpcStoreTransaction {
                npc_entity_id,
                buy_items,
                sell_items,
            } => {
                connection
                    .write_packet(Packet::from(&PacketClientNpcStoreTransaction {
                        npc_entity_id,
                        buy_items,
                        sell_items,
                    }))
                    .await?
            }
            ClientMessage::PartyCreate { invited_entity_id } => {
                connection
                    .write_packet(Packet::from(&PacketClientPartyRequest::Create(
                        invited_entity_id,
                    )))
                    .await?
            }
            ClientMessage::PartyInvite { invited_entity_id } => {
                connection
                    .write_packet(Packet::from(&PacketClientPartyRequest::Invite(
                        invited_entity_id,
                    )))
                    .await?
            }
            ClientMessage::PartyLeave => {
                connection
                    .write_packet(Packet::from(&PacketClientPartyRequest::Leave))
                    .await?
            }
            ClientMessage::PartyChangeOwner {
                new_owner_entity_id,
            } => {
                connection
                    .write_packet(Packet::from(&PacketClientPartyRequest::ChangeOwner(
                        new_owner_entity_id,
                    )))
                    .await?
            }
            ClientMessage::PartyKick { character_id } => {
                connection
                    .write_packet(Packet::from(&PacketClientPartyRequest::Kick(character_id)))
                    .await?
            }
            ClientMessage::PartyAcceptCreateInvite { owner_entity_id } => {
                connection
                    .write_packet(Packet::from(&PacketClientPartyReply::AcceptCreate(
                        owner_entity_id,
                    )))
                    .await?
            }
            ClientMessage::PartyAcceptJoinInvite { owner_entity_id } => {
                connection
                    .write_packet(Packet::from(&PacketClientPartyReply::AcceptJoin(
                        owner_entity_id,
                    )))
                    .await?
            }
            ClientMessage::PartyRejectInvite {
                reason,
                owner_entity_id,
            } => {
                connection
                    .write_packet(Packet::from(&PacketClientPartyReply::Reject(
                        reason,
                        owner_entity_id,
                    )))
                    .await?
            }
            ClientMessage::PartyUpdateRules {
                item_sharing,
                xp_sharing,
            } => {
                connection
                    .write_packet(Packet::from(&PacketClientPartyUpdateRules {
                        item_sharing,
                        xp_sharing,
                    }))
                    .await?
            }
            ClientMessage::MoveCollision { position } => {
                connection
                    .write_packet(Packet::from(&PacketClientMoveCollision { position }))
                    .await?
            }
            ClientMessage::PersonalStoreBuyItem {
                store_entity_id,
                store_slot_index,
                buy_item,
            } => {
                connection
                    .write_packet(Packet::from(&PacketClientPersonalStoreBuyItem {
                        store_entity_id,
                        store_slot_index,
                        buy_item,
                    }))
                    .await?
            }
            ClientMessage::BankOpen => {
                connection
                    .write_packet(Packet::from(&PacketClientBankOpen {}))
                    .await?
            }
            ClientMessage::BankDepositItem {
                item_slot,
                item,
                is_premium,
            } => {
                connection
                    .write_packet(Packet::from(&PacketClientBankMoveItem::Deposit {
                        item_slot,
                        item,
                        is_premium,
                    }))
                    .await?
            }
            ClientMessage::BankWithdrawItem {
                bank_slot,
                item,
                is_premium,
            } => {
                connection
                    .write_packet(Packet::from(&PacketClientBankMoveItem::Withdraw {
                        bank_slot,
                        item,
                        is_premium,
                    }))
                    .await?
            }
            ClientMessage::SetReviveSaveZone => {
                connection
                    .write_packet(Packet::from(&PacketClientSetReviveZone))
                    .await?
            }
            ClientMessage::ClanCreate {
                name,
                description,
                mark,
            } => {
                connection
                    .write_packet(Packet::from(&PacketClientClanCommand::Create {
                        name,
                        description,
                        mark,
                    }))
                    .await?;
            }
            ClientMessage::CraftInsertGem {
                equipment_index,
                item_slot,
            } => {
                connection
                    .write_packet(Packet::from(&PacketClientCraftItem::InsertGem {
                        equipment_index,
                        item_slot,
                    }))
                    .await?;
            }
            ClientMessage::CraftSkillDisassemble {
                skill_slot,
                item_slot,
            } => {
                connection
                    .write_packet(Packet::from(&PacketClientCraftItem::SkillDisassemble {
                        skill_slot,
                        item_slot,
                    }))
                    .await?;
            }
            ClientMessage::CraftNpcDisassemble {
                npc_entity_id,
                item_slot,
            } => {
                connection
                    .write_packet(Packet::from(&PacketClientCraftItem::NpcDisassemble {
                        npc_entity_id,
                        item_slot,
                    }))
                    .await?;
            }
            ClientMessage::CraftSkillUpgradeItem {
                skill_slot,
                item_slot,
                ingredients,
            } => {
                connection
                    .write_packet(Packet::from(&PacketClientCraftItem::SkillUpgradeItem {
                        skill_slot,
                        item_slot,
                        ingredients,
                    }))
                    .await?;
            }
            ClientMessage::CraftNpcUpgradeItem {
                npc_entity_id,
                item_slot,
                ingredients,
            } => {
                connection
                    .write_packet(Packet::from(&PacketClientCraftItem::NpcUpgradeItem {
                        npc_entity_id,
                        item_slot,
                        ingredients,
                    }))
                    .await?;
            }
            ClientMessage::RepairItemUsingItem {
                use_item_slot,
                item_slot,
            } => {
                connection
                    .write_packet(Packet::from(&PacketClientRepairItemUsingItem {
                        use_item_slot,
                        item_slot,
                    }))
                    .await?;
            }
            ClientMessage::RepairItemUsingNpc {
                npc_entity_id,
                item_slot,
            } => {
                connection
                    .write_packet(Packet::from(&PacketClientRepairItemUsingNpc {
                        npc_entity_id,
                        item_slot,
                    }))
                    .await?;
            }
            unimplemented => {
                log::info!("Unimplemented GameClient ClientMessage {:?}", unimplemented);
            }
        }
        Ok(())
    }
}

implement_protocol_client! { GameClient }
