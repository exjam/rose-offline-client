use std::time::Duration;

use bevy::{
    ecs::query::WorldQuery,
    math::Vec3Swizzles,
    prelude::{Entity, EventReader, EventWriter, Query, Res, With},
};

use rose_data::{
    AmmoIndex, EquipmentIndex, ItemClass, ItemType, SkillBasicCommand, SkillType, VehiclePartIndex,
};
use rose_game_common::{
    components::{Hotbar, HotbarSlot, Inventory, ItemDrop, SkillList, Team},
    messages::client::{Attack, ChangeEquipment, ClientMessage, Move, SetHotbarSlot},
};

use crate::{
    components::{
        ClientEntity, ConsumableCooldownGroup, Cooldowns, PartyInfo, PlayerCharacter, Position,
    },
    events::{ChatboxEvent, PlayerCommandEvent},
    resources::{GameConnection, GameData, SelectedTarget},
};

#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct PlayerQuery<'w> {
    _player_character: With<PlayerCharacter>,
    entity: Entity,
    cooldowns: &'w mut Cooldowns,
    hotbar: &'w mut Hotbar,
    inventory: &'w Inventory,
    position: &'w Position,
    skill_list: &'w SkillList,
    team: &'w Team,
    party_info: Option<&'w PartyInfo>,
}

#[allow(clippy::too_many_arguments)]
pub fn player_command_system(
    mut player_command_events: EventReader<PlayerCommandEvent>,
    mut query_player: Query<PlayerQuery>,
    query_client_entity: Query<&ClientEntity>,
    query_dropped_items: Query<(&ClientEntity, &Position), With<ItemDrop>>,
    query_team: Query<(&ClientEntity, &Team)>,
    mut chatbox_events: EventWriter<ChatboxEvent>,
    game_connection: Option<Res<GameConnection>>,
    game_data: Res<GameData>,
    selected_target: Res<SelectedTarget>,
) {
    let query_player_result = query_player.get_single_mut();
    if query_player_result.is_err() {
        return;
    }
    let mut player = query_player_result.unwrap();

    for event in player_command_events.iter() {
        let mut event = event.clone();

        if let PlayerCommandEvent::UseHotbar(page, index) = event {
            if let Some(hotbar_slot) = player
                .hotbar
                .pages
                .get(page)
                .and_then(|page| page.get(index))
                .and_then(|slot| slot.as_ref())
            {
                match hotbar_slot {
                    HotbarSlot::Skill(skill_slot) => {
                        event = PlayerCommandEvent::UseSkill(*skill_slot);
                    }
                    HotbarSlot::Inventory(item_slot) => {
                        event = PlayerCommandEvent::UseItem(*item_slot);
                    }
                    unimplemented => {
                        log::warn!("Unimplemented use hotbar slot {:?}", unimplemented);
                    }
                }
            }
        }

        match event {
            PlayerCommandEvent::UseSkill(skill_slot) => {
                if let Some(skill_data) = player
                    .skill_list
                    .get_skill(skill_slot)
                    .and_then(|skill_id| game_data.skills.get_skill(skill_id))
                {
                    // TODO: Check skill cooldown
                    if player.cooldowns.has_global_cooldown() {
                        chatbox_events.send(ChatboxEvent::System("Waiting...".to_string()));
                        continue;
                    }

                    player
                        .cooldowns
                        .set_global_cooldown(Duration::from_millis(250));

                    match skill_data.skill_type {
                        SkillType::BasicAction => match &skill_data.basic_command {
                            Some(SkillBasicCommand::Sit) => {
                                if let Some(game_connection) = game_connection.as_ref() {
                                    game_connection
                                        .client_message_tx
                                        .send(ClientMessage::SitToggle)
                                        .ok();
                                }
                            }
                            Some(SkillBasicCommand::PickupItem) => {
                                let mut nearest_item_drop = None;

                                for (item_client_entity, item_position) in
                                    query_dropped_items.iter()
                                {
                                    let distance = item_position
                                        .position
                                        .xy()
                                        .distance_squared(player.position.xy());

                                    if nearest_item_drop
                                        .as_ref()
                                        .map_or(true, |(nearest_distance, _, _)| {
                                            distance < *nearest_distance
                                        })
                                    {
                                        nearest_item_drop =
                                            Some((distance, item_position, item_client_entity.id));
                                    }
                                }

                                if let Some((_, target_position, target_entity_id)) =
                                    nearest_item_drop
                                {
                                    if let Some(game_connection) = game_connection.as_ref() {
                                        game_connection
                                            .client_message_tx
                                            .send(ClientMessage::Move(Move {
                                                target_entity_id: Some(target_entity_id),
                                                x: target_position.x,
                                                y: target_position.y,
                                                z: target_position.z as u16,
                                            }))
                                            .ok();
                                    }
                                }
                            }
                            Some(SkillBasicCommand::Attack) => {
                                if let Some(selected_target_entity) = selected_target.selected {
                                    if let Ok((target_client_entity, target_team)) =
                                        query_team.get(selected_target_entity)
                                    {
                                        if target_team.id != Team::DEFAULT_NPC_TEAM_ID
                                            && target_team.id != player.team.id
                                        {
                                            if let Some(game_connection) = game_connection.as_ref()
                                            {
                                                game_connection
                                                    .client_message_tx
                                                    .send(ClientMessage::Attack(Attack {
                                                        target_entity_id: target_client_entity.id,
                                                    }))
                                                    .ok();
                                            }
                                        }
                                    }
                                }
                            }
                            Some(SkillBasicCommand::Jump) | Some(SkillBasicCommand::AirJump) => {
                                if let Some(action_motion_id) = skill_data.action_motion_id {
                                    if let Some(game_connection) = game_connection.as_ref() {
                                        game_connection
                                            .client_message_tx
                                            .send(ClientMessage::UseEmote(action_motion_id, true))
                                            .ok();
                                    }
                                }
                            }
                            Some(SkillBasicCommand::PartyInvite) => {
                                if let Some(selected_target_entity) = selected_target.selected {
                                    if let Ok((target_client_entity, target_team)) =
                                        query_team.get(selected_target_entity)
                                    {
                                        if target_team.id == player.team.id {
                                            if let Some(game_connection) = game_connection.as_ref()
                                            {
                                                let message = if player.party_info.is_none() {
                                                    ClientMessage::PartyCreate(
                                                        target_client_entity.id,
                                                    )
                                                } else {
                                                    ClientMessage::PartyInvite(
                                                        target_client_entity.id,
                                                    )
                                                };

                                                game_connection
                                                    .client_message_tx
                                                    .send(message)
                                                    .ok();
                                            }
                                        }
                                    }
                                }
                            }
                            /*
                            Some(SkillBasicCommand::AutoTarget) => {}
                            Some(SkillBasicCommand::DriveVehicle) => {}
                            Some(SkillBasicCommand::AddFriend) => {}
                            Some(SkillBasicCommand::Trade) => {}
                            Some(SkillBasicCommand::PrivateStore) => {}
                            Some(SkillBasicCommand::SelfTarget) => {}
                            Some(SkillBasicCommand::VehiclePassengerInvite) => {}
                            */
                            Some(unimplemented) => {
                                log::warn!(
                                    "Unimplemented skill basic command type: {:?}",
                                    unimplemented
                                );
                            }
                            None => {}
                        },

                        SkillType::Emote => {
                            if let Some(motion_id) = skill_data.action_motion_id {
                                if let Some(game_connection) = game_connection.as_ref() {
                                    game_connection
                                        .client_message_tx
                                        .send(ClientMessage::UseEmote(motion_id, true))
                                        .ok();
                                }
                            }
                        }

                        SkillType::CreateWindow => {
                            log::warn!("Unimplemented skill type: {:?}", skill_data.skill_type);
                        }

                        SkillType::SelfBoundDuration
                        | SkillType::SelfBound
                        | SkillType::SelfStateDuration
                        | SkillType::SummonPet
                        | SkillType::SelfDamage => {
                            if let Some(game_connection) = game_connection.as_ref() {
                                game_connection
                                    .client_message_tx
                                    .send(ClientMessage::CastSkillSelf(skill_slot))
                                    .ok();
                            }
                        }

                        SkillType::AreaTarget => {
                            log::warn!(
                                "Unimplemented target position skill type: {:?}",
                                skill_data.skill_type
                            );
                        }

                        SkillType::EnforceWeapon
                        | SkillType::Immediate
                        | SkillType::TargetBound
                        | SkillType::TargetBoundDuration
                        | SkillType::TargetStateDuration
                        | SkillType::SelfAndTarget
                        | SkillType::Resurrection
                        | SkillType::EnforceBullet
                        | SkillType::FireBullet => {
                            // TODO: Check target team
                            if let Some((target_client_entity, _)) = selected_target
                                .selected
                                .and_then(|selected_entity| query_team.get(selected_entity).ok())
                            {
                                if let Some(game_connection) = game_connection.as_ref() {
                                    game_connection
                                        .client_message_tx
                                        .send(ClientMessage::CastSkillTargetEntity(
                                            skill_slot,
                                            target_client_entity.id,
                                        ))
                                        .ok();
                                }
                            } else {
                                chatbox_events
                                    .send(ChatboxEvent::System("Invalid target".to_string()));
                                continue;
                            }
                        }

                        SkillType::Passive => {} // Do nothing for passive skills
                        SkillType::Warp => {} // Warp skill is only used on items, so we should never hit it here
                    }
                }
            }
            PlayerCommandEvent::UseItem(item_slot) => {
                if let Some(item) = player.inventory.get_item(item_slot) {
                    if item.get_item_type() == ItemType::Consumable {
                        let consumable_item_data =
                            game_data.items.get_consumable_item(item.get_item_number());
                        let mut use_item_target = None;

                        if let Some(consumable_item_data) = consumable_item_data {
                            let cooldown_group = ConsumableCooldownGroup::from_item(
                                &item.get_item_reference(),
                                &game_data,
                            );
                            let cooldown_duration = match cooldown_group {
                                Some(ConsumableCooldownGroup::MagicItem) => {
                                    Some(Duration::from_millis(3000))
                                }
                                Some(_) => Some(Duration::from_millis(500)),
                                None => todo!(),
                            };

                            // TODO: If item is a repair item, we need to handle this client side
                            if matches!(consumable_item_data.item_data.class, ItemClass::RepairTool)
                            {
                                log::info!("TODO: Implement using ItemClass::RepairTool");
                                continue;
                            }

                            if matches!(
                                consumable_item_data.item_data.class,
                                ItemClass::QuestScroll
                            ) {
                                // TODO: This should open a dialog
                                log::info!("TODO: Implement using ItemClass::QuestScroll");
                                continue;
                            }

                            // Check if item is on cooldown
                            if cooldown_group
                                .and_then(|cooldown_group| {
                                    player
                                        .cooldowns
                                        .get_consumable_cooldown_percent(cooldown_group)
                                })
                                .is_some()
                            {
                                chatbox_events.send(ChatboxEvent::System("Waiting...".to_string()));
                                continue;
                            }

                            // Check if consumable requires a target
                            if matches!(consumable_item_data.item_data.class, ItemClass::MagicItem)
                            {
                                if let Some(skill_data) = consumable_item_data
                                    .use_skill_id
                                    .and_then(|skill_id| game_data.skills.get_skill(skill_id))
                                {
                                    if matches!(
                                        skill_data.skill_type,
                                        SkillType::FireBullet
                                            | SkillType::TargetBoundDuration
                                            | SkillType::TargetBound
                                            | SkillType::TargetStateDuration
                                    ) {
                                        if let Some((target_client_entity, _)) =
                                            selected_target.selected.and_then(|target_entity| {
                                                query_team.get(target_entity).ok()
                                            })
                                        {
                                            // TODO: Check target team
                                            use_item_target = Some(target_client_entity.id);
                                        } else {
                                            chatbox_events.send(ChatboxEvent::System(
                                                "Invalid target".to_string(),
                                            ));
                                            continue;
                                        }
                                    }
                                }
                            }

                            if let (Some(cooldown_group), Some(cooldown_duration)) =
                                (cooldown_group, cooldown_duration)
                            {
                                player
                                    .cooldowns
                                    .set_consumable_cooldown(cooldown_group, cooldown_duration);
                            }

                            if let Some(game_connection) = game_connection.as_ref() {
                                game_connection
                                    .client_message_tx
                                    .send(ClientMessage::UseItem(item_slot, use_item_target))
                                    .ok();
                            }
                        }
                    } else if item.get_item_type().is_equipment_item() {
                        // TODO: Equip item
                    }
                }
            }
            PlayerCommandEvent::EquipAmmo(item_slot) => {
                if let Some(item) = player.inventory.get_item(item_slot) {
                    let ammo_index = if let Some(item_data) =
                        game_data.items.get_base_item(item.get_item_reference())
                    {
                        match item_data.class {
                            ItemClass::Arrow => Some(AmmoIndex::Arrow),
                            ItemClass::Bullet => Some(AmmoIndex::Bullet),
                            ItemClass::Shell => Some(AmmoIndex::Throw),
                            _ => None,
                        }
                    } else {
                        None
                    };

                    if let Some(ammo_index) = ammo_index {
                        if let Some(game_connection) = game_connection.as_ref() {
                            game_connection
                                .client_message_tx
                                .send(ClientMessage::ChangeAmmo(ammo_index, Some(item_slot)))
                                .ok();
                        }
                    }
                }
            }
            PlayerCommandEvent::EquipEquipment(item_slot) => {
                if let Some(item) = player.inventory.get_item(item_slot) {
                    let equipment_index = match item.get_item_type() {
                        ItemType::Face => Some(EquipmentIndex::Face),
                        ItemType::Head => Some(EquipmentIndex::Head),
                        ItemType::Body => Some(EquipmentIndex::Body),
                        ItemType::Hands => Some(EquipmentIndex::Hands),
                        ItemType::Feet => Some(EquipmentIndex::Feet),
                        ItemType::Back => Some(EquipmentIndex::Back),
                        ItemType::Jewellery => {
                            if let Some(jewellery_item) =
                                game_data.items.get_jewellery_item(item.get_item_number())
                            {
                                match jewellery_item.item_data.class {
                                    ItemClass::Ring => Some(EquipmentIndex::Ring),
                                    ItemClass::Necklace => Some(EquipmentIndex::Necklace),
                                    ItemClass::Earring => Some(EquipmentIndex::Earring),
                                    _ => None,
                                }
                            } else {
                                None
                            }
                        }
                        ItemType::Weapon => Some(EquipmentIndex::Weapon),
                        ItemType::SubWeapon => Some(EquipmentIndex::SubWeapon),
                        _ => None,
                    };

                    if let Some(equipment_index) = equipment_index {
                        if let Some(game_connection) = game_connection.as_ref() {
                            game_connection
                                .client_message_tx
                                .send(ClientMessage::ChangeEquipment(ChangeEquipment {
                                    equipment_index,
                                    item_slot: Some(item_slot),
                                }))
                                .ok();
                        }
                    }
                }
            }
            PlayerCommandEvent::EquipVehicle(item_slot) => {
                if let Some(item) = player.inventory.get_item(item_slot) {
                    let vehicle_part_index = if let Some(item_data) =
                        game_data.items.get_base_item(item.get_item_reference())
                    {
                        match item_data.class {
                            ItemClass::CartBody | ItemClass::CastleGearBody => {
                                Some(VehiclePartIndex::Body)
                            }
                            ItemClass::CartEngine | ItemClass::CastleGearEngine => {
                                Some(VehiclePartIndex::Engine)
                            }
                            ItemClass::CartWheels | ItemClass::CastleGearLeg => {
                                Some(VehiclePartIndex::Leg)
                            }
                            ItemClass::CartAccessory | ItemClass::CastleGearWeapon => {
                                Some(VehiclePartIndex::Arms)
                            }
                            _ => None,
                        }
                    } else {
                        None
                    };

                    if let Some(vehicle_part_index) = vehicle_part_index {
                        if let Some(game_connection) = game_connection.as_ref() {
                            game_connection
                                .client_message_tx
                                .send(ClientMessage::ChangeVehiclePart(
                                    vehicle_part_index,
                                    Some(item_slot),
                                ))
                                .ok();
                        }
                    }
                }
            }
            PlayerCommandEvent::UnequipAmmo(ammo_index) => {
                if let Some(game_connection) = game_connection.as_ref() {
                    game_connection
                        .client_message_tx
                        .send(ClientMessage::ChangeAmmo(ammo_index, None))
                        .ok();
                }
            }
            PlayerCommandEvent::UnequipEquipment(equipment_index) => {
                if let Some(game_connection) = game_connection.as_ref() {
                    game_connection
                        .client_message_tx
                        .send(ClientMessage::ChangeEquipment(ChangeEquipment {
                            equipment_index,
                            item_slot: None,
                        }))
                        .ok();
                }
            }
            PlayerCommandEvent::UnequipVehicle(vehicle_part_index) => {
                if let Some(game_connection) = game_connection.as_ref() {
                    game_connection
                        .client_message_tx
                        .send(ClientMessage::ChangeVehiclePart(vehicle_part_index, None))
                        .ok();
                }
            }
            PlayerCommandEvent::DropItem(item_slot) => {
                if let Some(item) = player.inventory.get_item(item_slot) {
                    // TODO: if item.get_quantity() > 1, show dialog to pick how many to drop
                    if let Some(game_connection) = game_connection.as_ref() {
                        game_connection
                            .client_message_tx
                            .send(ClientMessage::DropItem(
                                item_slot,
                                item.get_quantity() as usize,
                            ))
                            .ok();
                    }
                }
            }
            PlayerCommandEvent::DropMoney(amount) => {
                if let Some(game_connection) = game_connection.as_ref() {
                    game_connection
                        .client_message_tx
                        .send(ClientMessage::DropMoney(amount))
                        .ok();
                }
            }
            PlayerCommandEvent::Attack(entity) => {
                if let Ok((target_client_entity, target_team)) = query_team.get(entity) {
                    if target_team.id != Team::DEFAULT_NPC_TEAM_ID
                        && target_team.id != player.team.id
                    {
                        if let Some(game_connection) = game_connection.as_ref() {
                            game_connection
                                .client_message_tx
                                .send(ClientMessage::Attack(Attack {
                                    target_entity_id: target_client_entity.id,
                                }))
                                .ok();
                        }
                    }
                }
            }
            PlayerCommandEvent::Move(position, target_entity) => {
                let target_entity_id = target_entity
                    .and_then(|target_entity| query_client_entity.get(target_entity).ok())
                    .map(|target_client_entity| target_client_entity.id);

                if let Some(game_connection) = game_connection.as_ref() {
                    game_connection
                        .client_message_tx
                        .send(ClientMessage::Move(Move {
                            target_entity_id,
                            x: position.x,
                            y: position.y,
                            z: position.z as u16,
                        }))
                        .ok();
                }
            }
            PlayerCommandEvent::SetHotbar(page, page_index, hotbar_slot) => {
                if let Some(hotbar_page) = player.hotbar.pages.get_mut(page) {
                    if let Some(hotbar_page_slot) = hotbar_page.get_mut(page_index) {
                        *hotbar_page_slot = hotbar_slot.clone();
                    }
                }

                if let Some(game_connection) = game_connection.as_ref() {
                    game_connection
                        .client_message_tx
                        .send(ClientMessage::SetHotbarSlot(SetHotbarSlot {
                            slot_index: page * player.hotbar.pages[0].len() + page_index,
                            slot: hotbar_slot,
                        }))
                        .ok();
                }
            }
            PlayerCommandEvent::UseHotbar(_, _) => {} // Handled above
        }
    }
}
