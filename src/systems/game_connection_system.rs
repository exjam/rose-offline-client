use arrayvec::ArrayVec;
use bevy::{
    ecs::event::Events,
    math::{Quat, Vec3},
    prelude::{
        Commands, ComputedVisibility, DespawnRecursiveExt, Entity, EventWriter, GlobalTransform,
        Mut, Res, ResMut, State, Transform, Visibility, World,
    },
};

use rose_data::{
    AbilityType, EquipmentItem, Item, ItemReference, ItemSlotBehaviour, ItemType, StatusEffectType,
};
use rose_game_common::{
    components::{
        AbilityValues, BasicStatType, BasicStats, CharacterInfo, DroppedItem, Equipment,
        ExperiencePoints, HealthPoints, Hotbar, Inventory, ItemDrop, ItemSlot, Level, ManaPoints,
        Money, MoveMode, MoveSpeed, QuestState, SkillList, Stamina, StatPoints, StatusEffects,
        StatusEffectsRegen,
    },
    messages::{
        server::{
            CommandState, LearnSkillError, LevelUpSkillError, MoveToggle, OpenPersonalStore,
            PartyMemberInfo, PartyMemberInfoOffline, PartyMemberLeave, PartyMemberList,
            PersonalStoreTransactionStatus, PickupItemDropContent, PickupItemDropError,
            QuestDeleteResult, QuestTriggerResult, ServerMessage, UpdateAbilityValue,
        },
        PartyItemSharing, PartyXpSharing,
    },
};
use rose_network_common::ConnectionError;

use crate::{
    bundles::{ability_values_add_value_exclusive, ability_values_set_value_exclusive},
    components::{
        Bank, ClientEntity, ClientEntityName, ClientEntityType, CollisionHeightOnly,
        CollisionPlayer, Command, CommandCastSkillTarget, Cooldowns, FacingDirection, NextCommand,
        PartyInfo, PartyOwner, PassiveRecoveryTime, PendingDamage, PendingDamageList,
        PendingSkillEffect, PendingSkillEffectList, PendingSkillTarget, PendingSkillTargetList,
        PersonalStore, PlayerCharacter, Position, VisibleStatusEffects,
    },
    events::{
        BankEvent, ChatboxEvent, ClientEntityEvent, GameConnectionEvent, LoadZoneEvent, PartyEvent,
        PersonalStoreEvent, QuestTriggerEvent,
    },
    resources::{AppState, ClientEntityList, GameConnection, GameData, WorldRates, WorldTime},
};

fn update_inventory_and_money(
    world: &mut World,
    player_entity: Entity,
    update_items: Vec<(ItemSlot, Option<Item>)>,
    update_money: Option<Money>,
) {
    let mut player = world.entity_mut(player_entity);

    if let Some(mut inventory) = player.get_mut::<Inventory>() {
        for (item_slot, item) in update_items.iter() {
            if let ItemSlot::Inventory(_, _) = item_slot {
                if let Some(item_slot) = inventory.get_item_slot_mut(*item_slot) {
                    *item_slot = item.clone();
                }
            }
        }

        if let Some(money) = update_money {
            inventory.money = money;
        }
    }

    if let Some(mut equipment) = player.get_mut::<Equipment>() {
        for (item_slot, item) in update_items.iter() {
            match *item_slot {
                ItemSlot::Ammo(ammo_index) => {
                    *equipment.get_ammo_slot_mut(ammo_index) =
                        item.as_ref().and_then(|x| x.as_stackable().cloned())
                }
                ItemSlot::Equipment(equipment_index) => {
                    *equipment.get_equipment_slot_mut(equipment_index) =
                        item.as_ref().and_then(|x| x.as_equipment().cloned())
                }
                ItemSlot::Vehicle(vehicle_part_index) => {
                    *equipment.get_vehicle_slot_mut(vehicle_part_index) =
                        item.as_ref().and_then(|x| x.as_equipment().cloned())
                }
                _ => {}
            }
        }
    }
}

pub fn game_connection_system(
    mut commands: Commands,
    game_connection: Option<Res<GameConnection>>,
    game_data: Res<GameData>,
    mut app_state: ResMut<State<AppState>>,
    mut client_entity_list: ResMut<ClientEntityList>,
    mut chatbox_events: EventWriter<ChatboxEvent>,
    mut game_connection_events: EventWriter<GameConnectionEvent>,
    mut load_zone_events: EventWriter<LoadZoneEvent>,
    mut client_entity_events: EventWriter<ClientEntityEvent>,
    mut party_events: EventWriter<PartyEvent>,
    mut personal_store_events: EventWriter<PersonalStoreEvent>,
    mut quest_trigger_events: EventWriter<QuestTriggerEvent>,
) {
    if game_connection.is_none() {
        return;
    }
    let game_connection = game_connection.unwrap();

    let result: Result<(), anyhow::Error> = loop {
        match game_connection.server_message_rx.try_recv() {
            Ok(ServerMessage::ConnectionResponse(response)) => match response {
                Ok(_) => {
                    client_entity_list.clear();
                }
                Err(_) => {
                    break Err(ConnectionError::ConnectionLost.into());
                }
            },
            Ok(ServerMessage::CharacterData(character_data)) => {
                let status_effects = StatusEffects::default();
                let ability_values = game_data.ability_value_calculator.calculate(
                    &character_data.character_info,
                    &character_data.level,
                    &character_data.equipment,
                    &character_data.basic_stats,
                    &character_data.skill_list,
                    &status_effects,
                );
                let move_mode = MoveMode::Run;
                let move_speed = MoveSpeed::new(ability_values.get_move_speed(&move_mode));

                // Spawn character
                client_entity_list.player_entity = Some(
                    commands
                        .spawn_bundle((
                            PlayerCharacter {},
                            ClientEntityName::new(character_data.character_info.name.clone()),
                            character_data.character_info,
                            character_data.basic_stats,
                            character_data.level,
                            character_data.equipment,
                            character_data.experience_points,
                            character_data.skill_list,
                            character_data.hotbar,
                            character_data.health_points,
                            character_data.mana_points,
                            character_data.stat_points,
                            character_data.skill_points,
                            character_data.union_membership,
                            character_data.stamina,
                        ))
                        .insert_bundle((
                            Command::with_stop(),
                            NextCommand::with_stop(),
                            FacingDirection::default(),
                            ability_values,
                            status_effects,
                            StatusEffectsRegen::new(),
                            move_mode,
                            move_speed,
                            Cooldowns::default(),
                            PassiveRecoveryTime::default(),
                            PendingSkillTargetList::default(),
                            PendingDamageList::default(),
                            PendingSkillEffectList::default(),
                            Position::new(character_data.position),
                            VisibleStatusEffects::default(),
                        ))
                        .insert_bundle((
                            Transform::from_xyz(
                                character_data.position.x / 100.0,
                                character_data.position.z / 100.0 + 100.0,
                                -character_data.position.y / 100.0,
                            ),
                            GlobalTransform::default(),
                            Visibility::default(),
                            ComputedVisibility::default(),
                        ))
                        .id(),
                );

                // Emit connected event, character select system will be responsible for
                // starting the load of the next zone once its animations have completed
                game_connection_events.send(GameConnectionEvent::Connected(character_data.zone_id));
                client_entity_list.zone_id = Some(character_data.zone_id);
            }
            Ok(ServerMessage::CharacterDataItems(message)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands
                        .entity(player_entity)
                        .insert_bundle((message.inventory, message.equipment));
                }
            }
            Ok(ServerMessage::CharacterDataQuest(message)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.entity(player_entity).insert(message.quest_state);
                }
            }
            Ok(ServerMessage::JoinZone(message)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.entity(player_entity).insert_bundle((
                        ClientEntity::new(message.entity_id, ClientEntityType::Character),
                        CollisionPlayer,
                        Command::with_stop(),
                        NextCommand::with_stop(),
                        FacingDirection::default(),
                        message.experience_points,
                        message.team,
                        message.health_points,
                        message.mana_points,
                    ));

                    commands.insert_resource(WorldRates {
                        craft_rate: message.craft_rate,
                        item_price_rate: message.item_price_rate,
                        world_price_rate: message.world_price_rate,
                        town_price_rate: message.town_price_rate,
                    });
                    commands.insert_resource(WorldTime::new(message.world_ticks));

                    client_entity_list.clear();
                    client_entity_list.add(message.entity_id, player_entity);
                    client_entity_list.player_entity_id = Some(message.entity_id);

                    // Transition to in game state if we are not already
                    if !matches!(app_state.current(), AppState::Game) {
                        app_state.set(AppState::Game).ok();
                    }
                }
            }
            Ok(ServerMessage::SpawnEntityCharacter(message)) => {
                let status_effects = StatusEffects {
                    active: message.status_effects,
                    ..Default::default()
                };
                let target_entity = message
                    .target_entity_id
                    .and_then(|id| client_entity_list.get(id));
                let next_command = match message.command {
                    CommandState::Move => {
                        if let Some(destination) = message.destination {
                            NextCommand::with_move(destination, target_entity, None)
                        } else {
                            NextCommand::default()
                        }
                    }
                    CommandState::Attack => {
                        if let Some(target_entity) = target_entity {
                            NextCommand::with_attack(target_entity)
                        } else {
                            NextCommand::default()
                        }
                    }
                    CommandState::PersonalStore => NextCommand::with_personal_store(),
                    CommandState::Die => NextCommand::with_die(),
                    _ => NextCommand::default(),
                };
                let mut ability_values = game_data.ability_value_calculator.calculate(
                    &message.character_info,
                    &message.level,
                    &message.equipment,
                    &BasicStats::default(),
                    &SkillList::default(),
                    &status_effects,
                );
                ability_values.run_speed = message.move_speed.speed;
                ability_values.attack_speed += message.passive_attack_speed;
                ability_values.passive_attack_speed = message.passive_attack_speed;

                let entity = commands
                    .spawn_bundle((
                        ClientEntityName::new(message.character_info.name.clone()),
                        Command::with_stop(),
                        next_command,
                        message.character_info,
                        message.team,
                        message.health,
                        message.move_mode,
                        Position::new(message.position),
                        message.equipment,
                        message.level,
                        message.move_speed,
                        ability_values,
                        status_effects,
                        StatusEffectsRegen::new(),
                    ))
                    .insert_bundle((
                        ClientEntity::new(message.entity_id, ClientEntityType::Character),
                        CollisionHeightOnly,
                        FacingDirection::default(),
                        PendingDamageList::default(),
                        PendingSkillEffectList::default(),
                        PendingSkillTargetList::default(),
                        Transform::from_xyz(
                            message.position.x / 100.0,
                            message.position.z / 100.0 + 10000.0,
                            -message.position.y / 100.0,
                        ),
                        GlobalTransform::default(),
                        Visibility::default(),
                        ComputedVisibility::default(),
                        VisibleStatusEffects::default(),
                    ))
                    .id();

                if let Some((skin, title)) = message.personal_store_info {
                    commands
                        .entity(entity)
                        .insert(PersonalStore::new(title, skin as usize));
                }

                client_entity_list.add(message.entity_id, entity);
            }
            Ok(ServerMessage::SpawnEntityNpc(message)) => {
                let status_effects = StatusEffects {
                    active: message.status_effects,
                    ..Default::default()
                };
                let ability_values = game_data
                    .ability_value_calculator
                    .calculate_npc(message.npc.id, &status_effects, None, None)
                    .unwrap();
                let move_speed = MoveSpeed::new(ability_values.get_move_speed(&message.move_mode));
                let level = Level::new(ability_values.get_level() as u32);
                let target_entity = message
                    .target_entity_id
                    .and_then(|id| client_entity_list.get(id));
                let next_command = match message.command {
                    CommandState::Move => {
                        if let Some(destination) = message.destination {
                            NextCommand::with_move(destination, target_entity, None)
                        } else {
                            NextCommand::default()
                        }
                    }
                    CommandState::Attack => {
                        if let Some(target_entity) = target_entity {
                            NextCommand::with_attack(target_entity)
                        } else {
                            NextCommand::default()
                        }
                    }
                    CommandState::Die => NextCommand::with_die(),
                    _ => NextCommand::default(),
                };

                let entity = commands
                    .spawn_bundle((
                        Command::with_stop(),
                        next_command,
                        message.npc,
                        message.team,
                        message.health,
                        message.move_mode,
                        Position::new(message.position),
                        ability_values,
                        level,
                        move_speed,
                        status_effects,
                    ))
                    .insert_bundle((
                        ClientEntity::new(message.entity_id, ClientEntityType::Npc),
                        CollisionHeightOnly,
                        FacingDirection::default(),
                        PendingDamageList::default(),
                        PendingSkillEffectList::default(),
                        PendingSkillTargetList::default(),
                        VisibleStatusEffects::default(),
                        Transform::from_xyz(
                            message.position.x / 100.0,
                            message.position.z / 100.0 + 10000.0,
                            -message.position.y / 100.0,
                        )
                        .with_rotation(Quat::from_axis_angle(
                            Vec3::Y,
                            message.direction.to_radians(),
                        )),
                        GlobalTransform::default(),
                        Visibility::default(),
                        ComputedVisibility::default(),
                    ))
                    .id();

                client_entity_list.add(message.entity_id, entity);
            }
            Ok(ServerMessage::SpawnEntityMonster(message)) => {
                let status_effects = StatusEffects {
                    active: message.status_effects,
                    ..Default::default()
                };
                let ability_values = game_data
                    .ability_value_calculator
                    .calculate_npc(message.npc.id, &status_effects, None, None)
                    .unwrap();
                let move_speed = MoveSpeed::new(ability_values.get_move_speed(&message.move_mode));
                let level = Level::new(ability_values.get_level() as u32);
                let target_entity = message
                    .target_entity_id
                    .and_then(|id| client_entity_list.get(id));
                let next_command = match message.command {
                    CommandState::Move => {
                        if let Some(destination) = message.destination {
                            NextCommand::with_move(destination, target_entity, None)
                        } else {
                            NextCommand::default()
                        }
                    }
                    CommandState::Attack => {
                        if let Some(target_entity) = target_entity {
                            NextCommand::with_attack(target_entity)
                        } else {
                            NextCommand::default()
                        }
                    }
                    CommandState::Die => NextCommand::with_die(),
                    _ => NextCommand::default(),
                };
                let mut equipment = Equipment::new();
                if let Some(npc_data) = game_data.npcs.get_npc(message.npc.id) {
                    if npc_data.right_hand_part_index > 0 {
                        equipment
                            .equip_item(
                                EquipmentItem::new(
                                    ItemReference::new(
                                        ItemType::Weapon,
                                        npc_data.right_hand_part_index as usize,
                                    ),
                                    0,
                                )
                                .unwrap(),
                            )
                            .ok();
                    }

                    if npc_data.left_hand_part_index > 0 {
                        equipment
                            .equip_item(
                                EquipmentItem::new(
                                    ItemReference::new(
                                        ItemType::SubWeapon,
                                        npc_data.left_hand_part_index as usize,
                                    ),
                                    0,
                                )
                                .unwrap(),
                            )
                            .ok();
                    }
                }

                let entity = commands
                    .spawn_bundle((
                        Command::with_stop(),
                        next_command,
                        message.npc,
                        message.team,
                        message.health,
                        message.move_mode,
                        Position::new(message.position),
                        ability_values,
                        equipment,
                        level,
                        move_speed,
                        status_effects,
                    ))
                    .insert_bundle((
                        ClientEntity::new(message.entity_id, ClientEntityType::Monster),
                        CollisionHeightOnly,
                        FacingDirection::default(),
                        PendingDamageList::default(),
                        PendingSkillEffectList::default(),
                        PendingSkillTargetList::default(),
                        VisibleStatusEffects::default(),
                        Transform::from_xyz(
                            message.position.x / 100.0,
                            message.position.z / 100.0 + 10000.0,
                            -message.position.y / 100.0,
                        ),
                        GlobalTransform::default(),
                        Visibility::default(),
                        ComputedVisibility::default(),
                    ))
                    .id();

                client_entity_list.add(message.entity_id, entity);
            }
            Ok(ServerMessage::SpawnEntityItemDrop(message)) => {
                let name = match &message.dropped_item {
                    DroppedItem::Item(item) => game_data
                        .items
                        .get_base_item(item.get_item_reference())
                        .map(|item_data| item_data.name.to_string())
                        .unwrap_or_else(|| {
                            format!("[{:?} {}]", item.get_item_type(), item.get_item_number())
                        }),
                    DroppedItem::Money(money) => {
                        format!("{} Zuly", money.0)
                    }
                };

                // TODO: Use message.remaining_time, message.owner_entity_id ?
                let entity = commands
                    .spawn_bundle((
                        ClientEntityName::new(name),
                        ItemDrop::with_dropped_item(message.dropped_item),
                        Position::new(message.position),
                        ClientEntity::new(message.entity_id, ClientEntityType::ItemDrop),
                        CollisionHeightOnly,
                        Transform::from_xyz(
                            message.position.x / 100.0,
                            message.position.z / 100.0 + 10000.0,
                            -message.position.y / 100.0,
                        ),
                        GlobalTransform::default(),
                        Visibility::default(),
                        ComputedVisibility::default(),
                    ))
                    .id();

                client_entity_list.add(message.entity_id, entity);
            }
            Ok(ServerMessage::MoveEntity(message)) => {
                if let Some(entity) = client_entity_list.get(message.entity_id) {
                    let target_entity = message
                        .target_entity_id
                        .and_then(|id| client_entity_list.get(id));

                    commands.entity(entity).insert(NextCommand::with_move(
                        Vec3::new(message.x, message.y, message.z as f32),
                        target_entity,
                        message.move_mode,
                    ));
                }
            }
            Ok(ServerMessage::AdjustPosition(client_entity_id, position)) => {
                if let Some(entity) = client_entity_list.get(client_entity_id) {
                    commands
                        .entity(entity)
                        .insert(NextCommand::with_move(position, None, None));
                }
            }
            Ok(ServerMessage::StopMoveEntity(message)) => {
                if let Some(entity) = client_entity_list.get(message.entity_id) {
                    commands.entity(entity).insert(NextCommand::with_stop());
                }
            }
            Ok(ServerMessage::AttackEntity(message)) => {
                if let Some(entity) = client_entity_list.get(message.entity_id) {
                    if let Some(target_entity) = client_entity_list.get(message.target_entity_id) {
                        commands
                            .entity(entity)
                            .insert(NextCommand::with_attack(target_entity));
                    }
                }
            }
            Ok(ServerMessage::RemoveEntities(message)) => {
                for entity_id in message.entity_ids {
                    if let Some(entity) = client_entity_list.get(entity_id) {
                        client_entity_list.remove(entity_id);
                        commands.entity(entity).despawn_recursive();
                    }
                }
            }
            Ok(ServerMessage::DamageEntity(message)) => {
                if let Some(defender_entity) = client_entity_list.get(message.defender_entity_id) {
                    let killed_by_player = message.is_killed
                        && client_entity_list.player_entity
                            == client_entity_list.get(message.attacker_entity_id);

                    commands.add(move |world: &mut World| {
                        let mut defender = world.entity_mut(defender_entity);
                        if let Some(mut pending_damage_list) =
                            defender.get_mut::<PendingDamageList>()
                        {
                            pending_damage_list.push(PendingDamage::new(
                                message.attacker_entity_id,
                                message.damage,
                                message.is_killed,
                                message.is_immediate,
                                message.from_skill,
                            ));
                        }

                        if killed_by_player {
                            if let Some(name) = defender.get::<ClientEntityName>() {
                                let chat_message =
                                    format!("You have succeeded in hunting {}", name.as_str());
                                world
                                    .resource_mut::<Events<ChatboxEvent>>()
                                    .send(ChatboxEvent::System(chat_message));
                            }
                        }
                    });
                }
            }
            Ok(ServerMessage::Teleport(message)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    // Update player position
                    commands
                        .entity(player_entity)
                        .insert_bundle((
                            Position::new(Vec3::new(message.x, message.y, 0.0)),
                            Transform::from_xyz(message.x / 100.0, 100.0, -message.y / 100.0),
                        ))
                        .remove::<ClientEntity>()
                        .remove::<CollisionPlayer>();

                    // Despawn all non-player entities
                    for (client_entity_id, client_entity) in
                        client_entity_list.client_entities.iter().enumerate()
                    {
                        if let Some(client_entity) = client_entity {
                            if client_entity_list
                                .player_entity_id
                                .map_or(true, |id| id.0 != client_entity_id)
                            {
                                commands.entity(*client_entity).despawn_recursive();
                            }
                        }
                    }
                    client_entity_list.clear();

                    // Load next zone
                    load_zone_events.send(LoadZoneEvent::new(message.zone_id));
                    client_entity_list.zone_id = Some(message.zone_id);
                }
            }
            Ok(ServerMessage::LocalChat(message)) => {
                if let Some(chat_entity) = client_entity_list.get(message.entity_id) {
                    commands.add(move |world: &mut World| {
                        if let Some(name) = world.entity(chat_entity).get::<ClientEntityName>() {
                            let name = name.to_string();
                            world
                                .resource_mut::<Events<ChatboxEvent>>()
                                .send(ChatboxEvent::Say(name, message.text));
                        }
                    });
                }
            }
            Ok(ServerMessage::ShoutChat(message)) => {
                chatbox_events.send(ChatboxEvent::Shout(message.name, message.text));
            }
            Ok(ServerMessage::Whisper(message)) => {
                chatbox_events.send(ChatboxEvent::Whisper(message.from, message.text));
            }
            Ok(ServerMessage::AnnounceChat(message)) => {
                chatbox_events.send(ChatboxEvent::Announce(message.name, message.text));
            }
            Ok(ServerMessage::UpdateAbilityValue(message)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    match message {
                        UpdateAbilityValue::RewardAdd(ability_type, add_value) => {
                            chatbox_events.send(ChatboxEvent::System(format!(
                                "Ability {:?} has {} by {}.",
                                ability_type,
                                if add_value < 0 {
                                    "decreased"
                                } else {
                                    "increased"
                                },
                                add_value.abs(),
                            )));
                        }
                        UpdateAbilityValue::RewardSet(ability_type, set_value) => {
                            chatbox_events.send(ChatboxEvent::System(format!(
                                "Ability {:?} has been changed to {}.",
                                ability_type, set_value,
                            )));
                        }
                    }

                    commands.add(move |world: &mut World| {
                        let mut player = world.entity_mut(player_entity);
                        match message {
                            UpdateAbilityValue::RewardAdd(ability_type, add_value) => {
                                ability_values_add_value_exclusive(
                                    ability_type,
                                    add_value,
                                    &mut player,
                                );
                            }
                            UpdateAbilityValue::RewardSet(ability_type, set_value) => {
                                ability_values_set_value_exclusive(
                                    ability_type,
                                    set_value,
                                    &mut player,
                                );
                            }
                        }
                    });
                }
            }
            Ok(ServerMessage::UpdateAmmo(entity_id, ammo_index, item)) => {
                if let Some(entity) = client_entity_list.get(entity_id) {
                    commands.add(move |world: &mut World| {
                        if let Some(mut equipment) = world.entity_mut(entity).get_mut::<Equipment>()
                        {
                            if let Some(equipped_ammo) =
                                equipment.equipped_ammo[ammo_index].as_mut()
                            {
                                if let Some(item) = item {
                                    equipped_ammo.item = item.item;
                                } else {
                                    equipment.equipped_ammo[ammo_index] = None;
                                }
                            }
                        }
                    });
                }
            }
            Ok(ServerMessage::UpdateEquipment(message)) => {
                if let Some(entity) = client_entity_list.get(message.entity_id) {
                    commands.add(move |world: &mut World| {
                        if let Some(mut equipment) = world.entity_mut(entity).get_mut::<Equipment>()
                        {
                            if let Some(equipped_item) =
                                equipment.equipped_items[message.equipment_index].as_mut()
                            {
                                if let Some(item) = message.item {
                                    // Only update visual related data
                                    equipped_item.item = item.item;
                                    equipped_item.has_socket = item.has_socket;
                                    equipped_item.gem = item.gem;
                                    equipped_item.grade = item.grade;
                                } else {
                                    equipment.equipped_items[message.equipment_index] = None;
                                }
                            } else {
                                equipment.equipped_items[message.equipment_index] = message.item;
                            }
                        }
                    });
                }
            }
            Ok(ServerMessage::UpdateVehiclePart(message)) => {
                if let Some(entity) = client_entity_list.get(message.entity_id) {
                    commands.add(move |world: &mut World| {
                        if let Some(mut equipment) = world.entity_mut(entity).get_mut::<Equipment>()
                        {
                            if let Some(equipped_item) =
                                equipment.equipped_vehicle[message.vehicle_part_index].as_mut()
                            {
                                if let Some(item) = message.item {
                                    // Only update visual related data
                                    equipped_item.item = item.item;
                                    equipped_item.has_socket = item.has_socket;
                                    equipped_item.gem = item.gem;
                                    equipped_item.grade = item.grade;
                                } else {
                                    equipment.equipped_vehicle[message.vehicle_part_index] = None;
                                }
                            } else {
                                equipment.equipped_vehicle[message.vehicle_part_index] =
                                    message.item;
                            }
                        }
                    });
                }
            }
            Ok(ServerMessage::UpdateItemLife { item_slot, life }) => {
                if let Some(entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        match item_slot {
                            ItemSlot::Equipment(index) => {
                                if let Some(mut equipment) = world.entity_mut(entity).get_mut::<Equipment>() {
                                    if let Some(mut equipment_item) = equipment.get_equipment_item_mut(index) {
                                        equipment_item.life = life;
                                    }
                                }
                            },
                            ItemSlot::Vehicle(index) => {
                                if let Some(mut equipment) = world.entity_mut(entity).get_mut::<Equipment>() {
                                    if let Some(mut equipment_item) = equipment.get_vehicle_item_mut(index) {
                                        equipment_item.life = life;
                                    }
                                }
                            },
                            ItemSlot::Inventory(inventory_page, inventory_slot) => {
                                if let Some(mut inventory) =  world.entity_mut(entity).get_mut::<Inventory>() {
                                    if let Some(item) = inventory.get_item_mut(ItemSlot::Inventory(inventory_page, inventory_slot)) {
                                        match item {
                                            Item::Equipment(equipment_item) => equipment_item.life = life,
                                            Item::Stackable(_) => {},
                                        }
                                    }
                                }
                            },
                            ItemSlot::Ammo(_) => {},
                        }
                    });
                }
            }
            Ok(ServerMessage::UpdateInventory(update_items, update_money)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        update_inventory_and_money(
                            world,
                            player_entity,
                            update_items,
                            update_money,
                        );
                    });
                }
            }
            Ok(ServerMessage::UseInventoryItem(message)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        if let Some(mut inventory) =
                            world.entity_mut(player_entity).get_mut::<Inventory>()
                        {
                            if let Some(item_slot) =
                                inventory.get_item_slot_mut(message.inventory_slot)
                            {
                                item_slot.try_take_quantity(1);
                            }
                        }
                    });
                }
            }
            Ok(ServerMessage::UpdateMoney(money)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        if let Some(mut inventory) =
                            world.entity_mut(player_entity).get_mut::<Inventory>()
                        {
                            inventory.money = money;
                        }
                    });
                }
            }
            Ok(ServerMessage::UpdateBasicStat(message)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        world.resource_scope(|world, game_data: Mut<GameData>| {
                            let mut stat_point_cost = None;
                            let mut player = world.entity_mut(player_entity);

                            if let Some(mut basic_stats) = player.get_mut::<BasicStats>() {
                                let current_value = basic_stats.get(message.basic_stat_type);

                                // Calculate stat point cost if this looked like a user requested stat increase
                                if message.value == current_value + 1 {
                                    if let Some(cost) = game_data
                                        .ability_value_calculator
                                        .calculate_basic_stat_increase_cost(
                                            &basic_stats,
                                            message.basic_stat_type,
                                        )
                                    {
                                        stat_point_cost = Some(cost);
                                    }
                                }

                                // Update stats
                                match message.basic_stat_type {
                                    BasicStatType::Strength => basic_stats.strength = message.value,
                                    BasicStatType::Dexterity => {
                                        basic_stats.dexterity = message.value
                                    }
                                    BasicStatType::Intelligence => {
                                        basic_stats.intelligence = message.value
                                    }
                                    BasicStatType::Concentration => {
                                        basic_stats.concentration = message.value
                                    }
                                    BasicStatType::Charm => basic_stats.charm = message.value,
                                    BasicStatType::Sense => basic_stats.sense = message.value,
                                }
                            }

                            // Update stat points
                            if let Some(subtract_stat_points) = stat_point_cost {
                                if let Some(mut stat_points) = player.get_mut::<StatPoints>() {
                                    stat_points.points -=
                                        stat_points.points.min(subtract_stat_points);
                                }
                            }
                        });
                    });
                }
            }
            Ok(ServerMessage::UpdateLevel(message)) => {
                if let Some(entity) = client_entity_list.get(message.entity_id) {
                    client_entity_events.send(ClientEntityEvent::LevelUp(
                        entity,
                        Some(message.level.level),
                    ));

                    commands.entity(entity).insert_bundle((
                        message.level,
                        message.experience_points,
                        message.stat_points,
                        message.skill_points,
                    ));

                    // Update HP / MP to max for new level
                    commands.add(move |world: &mut World| {
                        world.resource_scope(|world, game_data: Mut<GameData>| {
                            let mut character = world.entity_mut(entity);

                            if let (
                                Some(basic_stats),
                                Some(character_info),
                                Some(equipment),
                                Some(skill_list),
                                Some(status_effects),
                            ) = (
                                character.get::<BasicStats>(),
                                character.get::<CharacterInfo>(),
                                character.get::<Equipment>(),
                                character.get::<SkillList>(),
                                character.get::<StatusEffects>(),
                            ) {
                                let ability_values = game_data.ability_value_calculator.calculate(
                                    character_info,
                                    &message.level,
                                    equipment,
                                    basic_stats,
                                    skill_list,
                                    status_effects,
                                );

                                if let Some(mut health_points) = character.get_mut::<HealthPoints>()
                                {
                                    health_points.hp = ability_values.get_max_health();
                                }

                                if let Some(mut mana_points) = character.get_mut::<ManaPoints>() {
                                    mana_points.mp = ability_values.get_max_health();
                                }
                            }
                        });
                    });
                }
            }
            Ok(ServerMessage::LevelUpEntity(client_entity_id)) => {
                if client_entity_list.player_entity_id == Some(client_entity_id) {
                    // Ignore, the server erroneously sends this message in addition to ServerMessage::UpdateLevel
                } else if let Some(entity) = client_entity_list.get(client_entity_id) {
                    client_entity_events.send(ClientEntityEvent::LevelUp(entity, None));

                    commands.add(move |world: &mut World| {
                        world.resource_scope(|world, game_data: Mut<GameData>| {
                            let mut character = world.entity_mut(entity);

                            // Update level
                            if let Some(mut level) = character.get_mut::<Level>() {
                                level.level += 1;
                            }

                            // Update HP / MP to max for new level
                            if let (
                                Some(basic_stats),
                                Some(character_info),
                                Some(equipment),
                                Some(level),
                                Some(skill_list),
                                Some(status_effects),
                            ) = (
                                character.get::<BasicStats>(),
                                character.get::<CharacterInfo>(),
                                character.get::<Equipment>(),
                                character.get::<Level>(),
                                character.get::<SkillList>(),
                                character.get::<StatusEffects>(),
                            ) {
                                let ability_values = game_data.ability_value_calculator.calculate(
                                    character_info,
                                    level,
                                    equipment,
                                    basic_stats,
                                    skill_list,
                                    status_effects,
                                );

                                if let Some(mut health_points) = character.get_mut::<HealthPoints>()
                                {
                                    health_points.hp = ability_values.get_max_health();
                                }

                                if let Some(mut mana_points) = character.get_mut::<ManaPoints>() {
                                    mana_points.mp = ability_values.get_max_health();
                                }
                            }
                        });
                    });
                }
            }
            Ok(ServerMessage::UpdateSpeed(message)) => {
                if let Some(entity) = client_entity_list.get(message.entity_id) {
                    commands
                        .entity(entity)
                        .insert(MoveSpeed::new(message.run_speed as f32));
                }
            }
            Ok(ServerMessage::UpdateStatusEffects(message)) => {
                if let Some(entity) = client_entity_list.get(message.entity_id) {
                    commands.add(move |world: &mut World| {
                        let mut entity_mut = world.entity_mut(entity);
                        let mut updated_hp = None;
                        let mut updated_mp = None;

                        // Clear StatusEffects for status effects which do not exist in the packet
                        if let Some(mut status_effects) = entity_mut.get_mut::<StatusEffects>() {
                            for (status_effect_type, active) in message.status_effects.iter() {
                                if active.is_some() {
                                    continue;
                                }

                                if status_effects.active[status_effect_type].is_some() {
                                    match status_effect_type {
                                        StatusEffectType::IncreaseHp => {
                                            updated_hp = message.updated_values.first().cloned();
                                        },
                                        StatusEffectType::IncreaseMp => {
                                            updated_mp = message.updated_values.last().cloned();
                                        },
                                        _ => {}
                                    }
                                    status_effects.active[status_effect_type] = None;
                                    status_effects.expire_times[status_effect_type] = None;
                                }
                            }
                        }

                        // Clear StatusEffectsRegen for status effects which do not exist in the packet
                        if let Some(mut status_effects_regen) = entity_mut.get_mut::<StatusEffectsRegen>() {
                            for (status_effect_type, active) in message.status_effects {
                                if active.is_some() {
                                    continue;
                                }

                                if status_effects_regen.regens[status_effect_type].is_some() {
                                    status_effects_regen.regens[status_effect_type] = None;
                                }
                            }
                        }

                        if let Some(updated_hp) = updated_hp {
                            if let Some(mut health_points) = entity_mut.get_mut::<HealthPoints>() {
                                health_points.hp = updated_hp;
                            }
                        }

                        if let Some(updated_mp) = updated_mp {
                            if let Some(mut mana_points) = entity_mut.get_mut::<ManaPoints>() {
                                mana_points.mp = updated_mp;
                            }
                        }
                    });
                }
            }
            Ok(ServerMessage::UpdateXpStamina(message)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        let mut player = world.entity_mut(player_entity);

                        if let Some(mut stamina) = player.get_mut::<Stamina>() {
                            stamina.stamina = message.stamina;
                        }

                        if let Some(mut experience_points) = player.get_mut::<ExperiencePoints>() {
                            let previous_xp = experience_points.xp;
                            experience_points.xp = message.xp;

                            if message.xp > previous_xp {
                                world.resource_mut::<Events<ChatboxEvent>>().send(
                                    ChatboxEvent::System(format!(
                                        "You have earned {} experience points.",
                                        message.xp - previous_xp
                                    )),
                                );
                            }
                        }
                    });
                }
            }
            Ok(ServerMessage::PickupItemDropResult(message)) => match message.result {
                Ok(PickupItemDropContent::Item(item_slot, item)) => {
                    if let Some(player_entity) = client_entity_list.player_entity {
                        if let Some(item_data) =
                            game_data.items.get_base_item(item.get_item_reference())
                        {
                            chatbox_events.send(ChatboxEvent::System(format!(
                                "You have earned {}.",
                                item_data.name
                            )));
                        }

                        commands.add(move |world: &mut World| {
                            let mut player = world.entity_mut(player_entity);
                            if let Some(mut inventory) = player.get_mut::<Inventory>() {
                                if let Some(inventory_slot) = inventory.get_item_slot_mut(item_slot)
                                {
                                    *inventory_slot = Some(item);
                                }
                            }
                        });
                    }
                }
                Ok(PickupItemDropContent::Money(money)) => {
                    if let Some(player_entity) = client_entity_list.player_entity {
                        chatbox_events.send(ChatboxEvent::System(format!(
                            "You have earned {} Zuly.",
                            money.0
                        )));

                        commands.add(move |world: &mut World| {
                            let mut player = world.entity_mut(player_entity);
                            if let Some(mut inventory) = player.get_mut::<Inventory>() {
                                inventory.try_add_money(money).ok();
                            }
                        });
                    }
                }
                Err(PickupItemDropError::InventoryFull) => {
                    chatbox_events.send(ChatboxEvent::System(
                        "Cannot pickup item, inventory full.".to_string(),
                    ));
                }
                Err(PickupItemDropError::NoPermission) => {
                    chatbox_events.send(ChatboxEvent::System(
                        "Cannot pickup item, it does not belong to you.".to_string(),
                    ));
                }
                Err(_) => {}
            },
            Ok(ServerMessage::RewardItems(items)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    for (_, item) in items.iter() {
                        if let Some(item_data) = item.as_ref().and_then(|item| {
                            game_data.items.get_base_item(item.get_item_reference())
                        }) {
                            chatbox_events.send(ChatboxEvent::System(format!(
                                "You have earned {}.",
                                item_data.name
                            )));
                        }
                    }

                    commands.add(move |world: &mut World| {
                        let mut player = world.entity_mut(player_entity);
                        if let Some(mut inventory) = player.get_mut::<Inventory>() {
                            for (item_slot, item) in items.into_iter() {
                                if let Some(inventory_slot) = inventory.get_item_slot_mut(item_slot)
                                {
                                    *inventory_slot = item;
                                }
                            }
                        }
                    });
                }
            }
            Ok(ServerMessage::RewardMoney(money)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    chatbox_events.send(ChatboxEvent::System(format!(
                        "You have earned {} Zuly.",
                        money.0
                    )));

                    commands.add(move |world: &mut World| {
                        let mut player = world.entity_mut(player_entity);
                        if let Some(mut inventory) = player.get_mut::<Inventory>() {
                            inventory.try_add_money(money).ok();
                        }
                    });
                }
            }
            Ok(ServerMessage::QuestDeleteResult(QuestDeleteResult {
                success,
                slot,
                quest_id,
            })) => {
                if success {
                    if let Some(player_entity) = client_entity_list.player_entity {
                        commands.add(move |world: &mut World| {
                            let mut player = world.entity_mut(player_entity);
                            if let Some(mut quest_state) = player.get_mut::<QuestState>() {
                                if let Some(active_quest) = quest_state.active_quests[slot].as_ref()
                                {
                                    if active_quest.quest_id == quest_id {
                                        quest_state.active_quests[slot] = None;
                                    }
                                }
                            }
                        });
                    }
                }
            }
            Ok(ServerMessage::QuestTriggerResult(QuestTriggerResult {
                success,
                trigger_hash,
            })) => {
                if success {
                    quest_trigger_events.send(QuestTriggerEvent::ApplyRewards(trigger_hash));
                }
            }
            Ok(ServerMessage::RunNpcDeathTrigger(npc_id)) => {
                if let Some(npc_data) = game_data.npcs.get_npc(npc_id) {
                    quest_trigger_events.send(QuestTriggerEvent::DoTrigger(
                        npc_data.death_quest_trigger_name.as_str().into(),
                    ));
                }
            }
            Ok(ServerMessage::SetHotbarSlot(slot_index, slot)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        let mut player = world.entity_mut(player_entity);
                        if let Some(mut hotbar) = player.get_mut::<Hotbar>() {
                            hotbar.set_slot(slot_index, slot);
                        }
                    });
                }
            }
            Ok(ServerMessage::LearnSkillResult(result)) => match result {
                Ok(message) => {
                    if let Some(player_entity) = client_entity_list.player_entity {
                        commands.add(move |world: &mut World| {
                            let mut player = world.entity_mut(player_entity);
                            if let Some(mut skill_list) = player.get_mut::<SkillList>() {
                                if let Some(skill_slot) =
                                    skill_list.get_slot_mut(message.skill_slot)
                                {
                                    *skill_slot = message.skill_id;
                                }
                            }
                        });

                        commands
                            .entity(player_entity)
                            .insert_bundle((message.updated_skill_points,));
                    }
                }
                Err(LearnSkillError::AlreadyLearnt) => chatbox_events.send(ChatboxEvent::System(
                    "Failed to learn skill, you already know it.".to_string(),
                )),
                Err(LearnSkillError::JobRequirement) => chatbox_events.send(ChatboxEvent::System(
                    "Failed to learn skill, you do not satisfy the job requirement.".to_string(),
                )),
                Err(LearnSkillError::SkillRequirement) => {
                    chatbox_events.send(ChatboxEvent::System(
                        "Failed to learn skill, you do not satisfy the skill requirement."
                            .to_string(),
                    ))
                }
                Err(LearnSkillError::AbilityRequirement) => {
                    chatbox_events.send(ChatboxEvent::System(
                        "Failed to learn skill, you do not satisfy the ability requirement."
                            .to_string(),
                    ))
                }
                Err(LearnSkillError::Full) => chatbox_events.send(ChatboxEvent::System(
                    "Failed to learn skill, you have too many skills.".to_string(),
                )),
                Err(LearnSkillError::InvalidSkillId) => chatbox_events.send(ChatboxEvent::System(
                    "Failed to learn skill, invalid skill.".to_string(),
                )),
                Err(LearnSkillError::SkillPointRequirement) => {
                    chatbox_events.send(ChatboxEvent::System(
                        "Failed to learn skill, not enough skill points.".to_string(),
                    ))
                }
            },
            Ok(ServerMessage::LevelUpSkillResult(message)) => {
                match message.result {
                    Ok((skill_slot, skill_id)) => {
                        if let Some(player_entity) = client_entity_list.player_entity {
                            commands.add(move |world: &mut World| {
                                let mut player = world.entity_mut(player_entity);
                                if let Some(mut skill_list) = player.get_mut::<SkillList>() {
                                    if let Some(skill_slot) = skill_list.get_slot_mut(skill_slot) {
                                        *skill_slot = Some(skill_id);
                                    }
                                }
                            });
                        }
                    }
                    Err(LevelUpSkillError::Failed) => chatbox_events.send(ChatboxEvent::System(
                        "Failed to level up skill.".to_string(),
                    )),
                    Err(LevelUpSkillError::JobRequirement) => {
                        chatbox_events.send(ChatboxEvent::System(
                            "Failed to level up skill, you do not satisfy the job requirement."
                                .to_string(),
                        ))
                    }
                    Err(LevelUpSkillError::SkillRequirement) => {
                        chatbox_events.send(ChatboxEvent::System(
                            "Failed to level up skill, you do not satisfy the skill requirement."
                                .to_string(),
                        ))
                    }
                    Err(LevelUpSkillError::AbilityRequirement) => {
                        chatbox_events.send(ChatboxEvent::System(
                            "Failed to level up skill, you do not satisfy the ability requirement."
                                .to_string(),
                        ))
                    }
                    Err(LevelUpSkillError::MoneyRequirement) => {
                        chatbox_events.send(ChatboxEvent::System(
                            "Failed to level up skill, not enough money.".to_string(),
                        ))
                    }
                    Err(LevelUpSkillError::SkillPointRequirement) => {
                        chatbox_events.send(ChatboxEvent::System(
                            "Failed to level up skill, not enough skill points.".to_string(),
                        ))
                    }
                }

                if let Some(player_entity) = client_entity_list.player_entity {
                    commands
                        .entity(player_entity)
                        .insert_bundle((message.updated_skill_points,));
                }
            }
            Ok(ServerMessage::UseEmote(message)) => {
                if let Some(entity) = client_entity_list.get(message.entity_id) {
                    let new_command = NextCommand::with_emote(message.motion_id, message.is_stop);
                    commands.entity(entity).insert(new_command);
                }
            }
            Ok(ServerMessage::SitToggle(entity_id)) => {
                if let Some(entity) = client_entity_list.get(entity_id) {
                    commands.add(move |world: &mut World| {
                        let mut character = world.entity_mut(entity);
                        let is_sitting =
                            matches!(character.get::<Command>(), Some(Command::Sit(_)));

                        if let Some(mut next_command) = character.get_mut::<NextCommand>() {
                            if is_sitting {
                                // If next command is already set then the command system will make the
                                // entity stand up before performing next command. So we only need to
                                // explicitly start to stand up if next command is not set.
                                if next_command.is_none() {
                                    *next_command = NextCommand::with_standing();
                                }
                            } else {
                                *next_command = NextCommand::with_sitting();
                            }
                        }
                    });
                }
            }
            Ok(ServerMessage::UseItem(message)) => {
                if let Some(entity) = client_entity_list.get(message.entity_id) {
                    client_entity_events.send(ClientEntityEvent::UseItem(entity, message.item));
                }
            }
            Ok(ServerMessage::CastSkillSelf(message)) => {
                if let Some(entity) = client_entity_list.get(message.entity_id) {
                    commands.entity(entity).insert(NextCommand::with_cast_skill(
                        message.skill_id,
                        None,
                        message.cast_motion_id,
                        None,
                        None,
                    ));
                }
            }
            Ok(ServerMessage::CastSkillTargetEntity(message)) => {
                if let Some(entity) = client_entity_list.get(message.entity_id) {
                    if let Some(target_entity) = client_entity_list.get(message.target_entity_id) {
                        commands.entity(entity).insert(NextCommand::with_cast_skill(
                            message.skill_id,
                            Some(CommandCastSkillTarget::Entity(target_entity)),
                            message.cast_motion_id,
                            None,
                            None,
                        ));
                    }
                }
            }
            Ok(ServerMessage::CastSkillTargetPosition(message)) => {
                if let Some(entity) = client_entity_list.get(message.entity_id) {
                    commands.entity(entity).insert(NextCommand::with_cast_skill(
                        message.skill_id,
                        Some(CommandCastSkillTarget::Position(message.target_position)),
                        message.cast_motion_id,
                        None,
                        None,
                    ));
                }
            }
            Ok(ServerMessage::CancelCastingSkill(entity_id, _)) => {
                if let Some(entity) = client_entity_list.get(entity_id) {
                    commands.add(move |world: &mut World| {
                        let mut character = world.entity_mut(entity);

                        if let Some(mut command) = character.get_mut::<Command>() {
                            if matches!(*command, Command::CastSkill(_)) {
                                *command = Command::with_stop();
                            }
                        }
                    });
                }
            }
            Ok(ServerMessage::StartCastingSkill(_entity_id)) => {
                // Nah bruv
            }
            Ok(ServerMessage::FinishCastingSkill(entity_id, skill_id)) => {
                if let Some(entity) = client_entity_list.get(entity_id) {
                    commands.add(move |world: &mut World| {
                        let mut character = world.entity_mut(entity);

                        if let Some(mut command) = character.get_mut::<Command>() {
                            if let Command::CastSkill(command_cast_skill) = command.as_mut() {
                                if command_cast_skill.skill_id == skill_id {
                                    command_cast_skill.ready_action = true;
                                    return;
                                }
                             }
                        }

                        if let Some(mut next_command) = character.get_mut::<NextCommand>() {
                            if let Some(Command::CastSkill(command_cast_skill)) = (*next_command).as_mut() {
                                if command_cast_skill.skill_id == skill_id {
                                    command_cast_skill.ready_action = true;
                                    return;
                                }
                            }
                        }

                        if let Some(command) = character.get::<Command>() {
                            if let Some(next_command) = character.get::<NextCommand>() {
                                log::error!("FinishCastingSkill entity was not in expected state, command: {:?}, next command: {:?}, expected CastSkill({:?})", *command, *next_command, skill_id);
                            }
                        }
                    });

                    if let Some(use_ability) = game_data
                        .skills
                        .get_skill(skill_id)
                        .map(|skill_data| skill_data.use_ability.clone())
                    {
                        let is_player = client_entity_list.player_entity == Some(entity);
                        commands.add(move |world: &mut World| {
                            let mut target = world.entity_mut(entity);

                            for (use_ability_type, mut use_ability_value) in use_ability {
                                // We only apply health point modification to other entities
                                if !is_player && use_ability_type != AbilityType::Health {
                                    continue;
                                }

                                if use_ability_type == AbilityType::Mana {
                                    if let Some(ability_values) = target.get::<AbilityValues>() {
                                        let use_mana_rate =
                                            (100 - ability_values.get_save_mana()) as f32 / 100.0;

                                        use_ability_value =
                                            (use_ability_value as f32 * use_mana_rate) as i32;
                                    }
                                }

                                ability_values_add_value_exclusive(
                                    use_ability_type,
                                    -use_ability_value,
                                    &mut target,
                                );
                            }
                        });
                    }
                }
            }
            Ok(ServerMessage::ApplySkillEffect(message)) => {
                if let Some(defender_entity) = client_entity_list.get(message.entity_id) {
                    let caster_entity = client_entity_list.get(message.caster_entity_id);

                    commands.add(move |world: &mut World| {
                        let mut defender = world.entity_mut(defender_entity);

                        if let Some(mut pending_skill_effect_list) =
                            defender.get_mut::<PendingSkillEffectList>()
                        {
                            pending_skill_effect_list.push(PendingSkillEffect::new(
                                message.skill_id,
                                caster_entity,
                                message.caster_intelligence,
                                message.effect_success,
                            ));
                        }

                        if let Some(caster_entity) = caster_entity {
                            if let Some(mut pending_skill_target_list) = world
                                .entity_mut(caster_entity)
                                .get_mut::<PendingSkillTargetList>()
                            {
                                pending_skill_target_list.push(PendingSkillTarget::new(
                                    message.skill_id,
                                    defender_entity,
                                ));
                            }
                        }
                    });
                }
            }
            Ok(ServerMessage::NpcStoreTransactionError(error)) => {
                chatbox_events.send(ChatboxEvent::System(format!(
                    "Store transation failed with error {:?}",
                    error
                )));
            }
            Ok(ServerMessage::PartyCreate(client_entity_id)) => {
                if let Some(inviter_entity) = client_entity_list.get(client_entity_id) {
                    party_events.send(PartyEvent::InvitedCreate(inviter_entity));
                }
            }
            Ok(ServerMessage::PartyInvite(client_entity_id)) => {
                if let Some(inviter_entity) = client_entity_list.get(client_entity_id) {
                    party_events.send(PartyEvent::InvitedJoin(inviter_entity));
                }
            }
            Ok(ServerMessage::PartyAcceptCreate(with_entity_id)) => {
                if let Some(invited_entity) = client_entity_list.get(with_entity_id) {
                    commands.add(move |world: &mut World| {
                        if let Some(invited_entity_name) =
                            world.entity(invited_entity).get::<ClientEntityName>()
                        {
                            let message = format!(
                                "{} accepted your party invite.",
                                invited_entity_name.as_str()
                            );
                            world
                                .resource_mut::<Events<ChatboxEvent>>()
                                .send(ChatboxEvent::System(message));
                        }
                    });
                }

                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.entity(player_entity).insert(PartyInfo {
                        owner: PartyOwner::Player,
                        ..Default::default()
                    });
                }
            }
            Ok(ServerMessage::PartyAcceptInvite(_)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.entity(player_entity).insert(PartyInfo {
                        owner: PartyOwner::Unknown,
                        ..Default::default()
                    });

                    commands.add(move |world: &mut World| {
                        if let Some(player_entity_name) =
                            world.entity(player_entity).get::<ClientEntityName>()
                        {
                            let message =
                                format!("{} has joined the party.", player_entity_name.as_str());
                            world
                                .resource_mut::<Events<ChatboxEvent>>()
                                .send(ChatboxEvent::System(message));
                        }
                    });
                }
            }
            Ok(ServerMessage::PartyRejectInvite(_reason, client_entity_id)) => {
                if let Some(invited_entity) = client_entity_list.get(client_entity_id) {
                    commands.add(move |world: &mut World| {
                        if let Some(invited_entity_name) =
                            world.entity(invited_entity).get::<ClientEntityName>()
                        {
                            let message = format!(
                                "{} rejected your party invite.",
                                invited_entity_name.as_str()
                            );
                            world
                                .resource_mut::<Events<ChatboxEvent>>()
                                .send(ChatboxEvent::System(message));
                        }
                    });
                }
            }
            Ok(ServerMessage::PartyChangeOwner(client_entity_id)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    let is_player_owner =
                        Some(client_entity_id) == client_entity_list.player_entity_id;

                    commands.add(move |world: &mut World| {
                        if let Some(mut party_info) =
                            world.entity_mut(player_entity).get_mut::<PartyInfo>()
                        {
                            if is_player_owner {
                                party_info.owner = PartyOwner::Player;

                                if let Some(character_info) =
                                    world.entity(player_entity).get::<CharacterInfo>()
                                {
                                    let message = format!(
                                        "{} is now leader of the party.",
                                        &character_info.name
                                    );
                                    world
                                        .resource_mut::<Events<ChatboxEvent>>()
                                        .send(ChatboxEvent::System(message));
                                }
                            } else {
                                party_info.owner = PartyOwner::Unknown;

                                for member in party_info.members.iter() {
                                    if let PartyMemberInfo::Online(member_info_online) = member {
                                        if member_info_online.entity_id == client_entity_id {
                                            let message = format!(
                                                "{} is now leader of the party.",
                                                &member_info_online.name
                                            );

                                            party_info.owner = PartyOwner::Character(
                                                member_info_online.character_id,
                                            );

                                            world
                                                .resource_mut::<Events<ChatboxEvent>>()
                                                .send(ChatboxEvent::System(message));
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    });
                }
            }
            Ok(ServerMessage::PartyDelete) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.entity(player_entity).remove::<PartyInfo>();
                    chatbox_events.send(ChatboxEvent::System("You have left the party.".into()));
                }
            }
            Ok(ServerMessage::PartyMemberList(PartyMemberList {
                mut members,
                item_sharing,
                xp_sharing,
                ..
            })) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        let mut player = world.entity_mut(player_entity);

                        if !player.contains::<PartyInfo>() {
                            player.insert(PartyInfo {
                                item_sharing,
                                xp_sharing,
                                ..Default::default()
                            });
                        }

                        let mut party_info = player.get_mut::<PartyInfo>().unwrap();
                        if matches!(party_info.owner, PartyOwner::Unknown) {
                            party_info.owner = PartyOwner::Character(members[0].get_character_id());
                        }

                        let mut messages: ArrayVec<String, 10> = ArrayVec::new();
                        for member in members.iter() {
                            messages.push(format!("{} has joined the party.", member.get_name()));
                        }

                        party_info.members.append(&mut members);

                        let mut chatbox_events = world.resource_mut::<Events<ChatboxEvent>>();
                        for message in messages {
                            chatbox_events.send(ChatboxEvent::System(message));
                        }
                    });
                }
            }
            Ok(ServerMessage::PartyMemberLeave(PartyMemberLeave {
                leaver_character_id,
                owner_character_id,
            })) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        let mut player = world.entity_mut(player_entity);
                        let player_unique_id =
                            player.get::<CharacterInfo>().map(|info| info.unique_id);

                        if let Some(mut party_info) = player.get_mut::<PartyInfo>() {
                            if player_unique_id == Some(owner_character_id) {
                                party_info.owner = PartyOwner::Player;
                            } else {
                                party_info.owner = PartyOwner::Character(owner_character_id);
                            }

                            if let Some(index) = party_info
                                .members
                                .iter()
                                .position(|x| x.get_character_id() == leaver_character_id)
                            {
                                let message = format!(
                                    "{} has left the party.",
                                    party_info.members[index].get_name()
                                );

                                party_info.members.remove(index);

                                world
                                    .resource_mut::<Events<ChatboxEvent>>()
                                    .send(ChatboxEvent::System(message));
                            }
                        }
                    });
                }
            }
            Ok(ServerMessage::PartyMemberDisconnect(character_unique_id)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        if let Some(mut party_info) =
                            world.entity_mut(player_entity).get_mut::<PartyInfo>()
                        {
                            if let Some(party_member) = party_info
                                .members
                                .iter_mut()
                                .find(|x| x.get_character_id() == character_unique_id)
                            {
                                if let PartyMemberInfo::Online(party_member_online) = party_member {
                                    let message =
                                        format!("{} has disconnected.", &party_member_online.name);

                                    *party_member =
                                        PartyMemberInfo::Offline(PartyMemberInfoOffline {
                                            character_id: party_member_online.character_id,
                                            name: party_member_online.name.clone(),
                                        });

                                    world
                                        .resource_mut::<Events<ChatboxEvent>>()
                                        .send(ChatboxEvent::System(message));
                                }
                            }
                        }
                    });
                }
            }
            Ok(ServerMessage::PartyMemberKicked(character_unique_id)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        if let Some(mut party_info) =
                            world.entity_mut(player_entity).get_mut::<PartyInfo>()
                        {
                            if let Some(index) = party_info
                                .members
                                .iter()
                                .position(|x| x.get_character_id() == character_unique_id)
                            {
                                let message = format!(
                                    "{} has been kicked from the party.",
                                    party_info.members[index].get_name()
                                );
                                party_info.members.remove(index);

                                world
                                    .resource_mut::<Events<ChatboxEvent>>()
                                    .send(ChatboxEvent::System(message));
                            }
                        }
                    });
                }
            }
            Ok(ServerMessage::PartyMemberUpdateInfo(party_member_info)) => {
                let member_entity = client_entity_list.get(party_member_info.entity_id);
                let player_entity = client_entity_list.player_entity;

                if member_entity.is_some() || player_entity.is_some() {
                    commands.add(move |world: &mut World| {
                        if let Some(mut member) = member_entity
                            .and_then(|member_entity| world.get_entity_mut(member_entity))
                        {
                            if let Some(mut basic_stats) = member.get_mut::<BasicStats>() {
                                basic_stats.concentration = party_member_info.concentration;
                            }

                            if let Some(mut health_points) = member.get_mut::<HealthPoints>() {
                                health_points.hp = party_member_info.health_points.hp;
                            }
                        }

                        if let Some(mut player) = player_entity
                            .and_then(|player_entity| world.get_entity_mut(player_entity))
                        {
                            if let Some(mut party_info) = player.get_mut::<PartyInfo>() {
                                if let Some(party_member) =
                                    party_info.members.iter_mut().find(|x| {
                                        x.get_character_id() == party_member_info.character_id
                                    })
                                {
                                    *party_member = PartyMemberInfo::Online(party_member_info);
                                }
                            }
                        }
                    });
                }
            }
            Ok(ServerMessage::PartyMemberRewardItem {
                client_entity_id,
                item,
            }) => {
                let member_entity = client_entity_list.get(client_entity_id);
                let item_name = game_data
                    .items
                    .get_base_item(item.get_item_reference())
                    .map(|item_data| item_data.name);

                if let (Some(member_entity), Some(item_name)) = (member_entity, item_name) {
                    commands.add(move |world: &mut World| {
                        if let Some(member) = world.get_entity(member_entity) {
                            if let Some(member_entity_name) = member.get::<ClientEntityName>() {
                                let chat_message = format!(
                                    "{} has earned {}.",
                                    member_entity_name.as_str(),
                                    item_name
                                );
                                world
                                    .resource_mut::<Events<ChatboxEvent>>()
                                    .send(ChatboxEvent::System(chat_message));
                            }
                        }
                    });
                }
            }
            Ok(ServerMessage::PartyUpdateRules(item_sharing, xp_sharing)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        if let Some(mut party_info) =
                            world.entity_mut(player_entity).get_mut::<PartyInfo>()
                        {
                            party_info.item_sharing = item_sharing;
                            party_info.xp_sharing = xp_sharing;

                            let mut chatbox_events = world.resource_mut::<Events<ChatboxEvent>>();
                            chatbox_events
                                .send(ChatboxEvent::System("Party rules have changed.".into()));
                            chatbox_events.send(ChatboxEvent::System(format!(
                                "Experience points sharing: {}.",
                                match xp_sharing {
                                    PartyXpSharing::EqualShare => "Equal Share",
                                    PartyXpSharing::DistributedByLevel => "Distributed by Level",
                                }
                            )));
                            chatbox_events.send(ChatboxEvent::System(format!(
                                "Item sharing: {}.",
                                match item_sharing {
                                    PartyItemSharing::EqualLootDistribution =>
                                        "Equal Loot Distribution",
                                    PartyItemSharing::AcquisitionOrder => "Acquisition Order",
                                }
                            )));
                        }
                    });
                }
            }
            Ok(ServerMessage::UpdateSkillList(skill_data)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        let mut player = world.entity_mut(player_entity);
                        if let Some(mut skill_list) = player.get_mut::<SkillList>() {
                            for update_skill in skill_data {
                                if let Some(skill_slot) =
                                    skill_list.get_slot_mut(update_skill.skill_slot)
                                {
                                    *skill_slot = update_skill.skill_id;
                                }
                            }
                        }
                    });
                }
            }
            Ok(ServerMessage::OpenPersonalStore(OpenPersonalStore {
                entity_id,
                skin,
                title,
            })) => {
                if let Some(entity) = client_entity_list.get(entity_id) {
                    commands.entity(entity).insert(PersonalStore {
                        title,
                        skin: skin as usize,
                    });
                }
            }
            Ok(ServerMessage::ClosePersonalStore(entity_id)) => {
                if let Some(entity) = client_entity_list.get(entity_id) {
                    commands.entity(entity).remove::<PersonalStore>();
                }
            }
            Ok(ServerMessage::PersonalStoreItemList(item_list)) => {
                personal_store_events.send(PersonalStoreEvent::SetItemList(item_list));
            }
            Ok(ServerMessage::PersonalStoreTransaction {
                status,
                store_entity_id,
                update_store,
            }) => {
                if !update_store.is_empty() {
                    if let Some(entity) = client_entity_list.get(store_entity_id) {
                        match status {
                            PersonalStoreTransactionStatus::Cancelled => {}
                            PersonalStoreTransactionStatus::SoldOut
                            | PersonalStoreTransactionStatus::BoughtFromStore => {
                                personal_store_events.send(PersonalStoreEvent::UpdateSellList {
                                    entity,
                                    item_list: update_store,
                                });
                            }
                            PersonalStoreTransactionStatus::NoMoreNeed
                            | PersonalStoreTransactionStatus::SoldToStore => {
                                personal_store_events.send(PersonalStoreEvent::UpdateBuyList {
                                    entity,
                                    item_list: update_store,
                                });
                            }
                        }
                    }
                }

                match status {
                    PersonalStoreTransactionStatus::Cancelled => {
                        chatbox_events
                            .send(ChatboxEvent::System("Transaction failed.".to_string()));
                    }
                    PersonalStoreTransactionStatus::SoldOut => {
                        chatbox_events.send(ChatboxEvent::System(
                            "Transaction failed. Item has sold out.".to_string(),
                        ));
                    }
                    PersonalStoreTransactionStatus::NoMoreNeed => {
                        chatbox_events.send(ChatboxEvent::System(
                            "Transaction failed. Item is no longer wanted.".to_string(),
                        ));
                    }
                    _ => {}
                }
            }
            Ok(ServerMessage::PersonalStoreTransactionUpdateInventory { money, items }) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        let player = world.entity(player_entity);

                        if let Some(inventory) = player.get::<Inventory>() {
                            let transaction_price = money.0 - inventory.money.0;

                            if let Some((item_slot, transaction_item)) = items.first() {
                                let transaction_item = transaction_item.as_ref();
                                let inventory_item = inventory.get_item(*item_slot);
                                let (transaction_quantity, transaction_item) =
                                    match (transaction_item, inventory_item) {
                                        (Some(transaction_item), Some(inventory_item)) => (
                                            transaction_item.get_quantity() as i32
                                                - inventory_item.get_quantity() as i32,
                                            Some(inventory_item.get_item_reference()),
                                        ),
                                        (None, Some(inventory_item)) => (
                                            inventory_item.get_quantity() as i32,
                                            Some(inventory_item.get_item_reference()),
                                        ),
                                        (Some(transaction_item), None) => (
                                            transaction_item.get_quantity() as i32,
                                            Some(transaction_item.get_item_reference()),
                                        ),
                                        (None, None) => (0, None),
                                    };

                                let game_data = world.resource::<GameData>();
                                if let Some(item_data) = transaction_item
                                    .and_then(|item| game_data.items.get_base_item(item))
                                {
                                    let message = if transaction_quantity > 1 {
                                        format!(
                                            "You have {} {}x {} for {} Zuly.",
                                            if transaction_price < 0 {
                                                "purchased"
                                            } else {
                                                "sold"
                                            },
                                            transaction_quantity,
                                            item_data.name,
                                            transaction_price.abs()
                                        )
                                    } else {
                                        format!(
                                            "You have {} {} for {} Zuly.",
                                            if transaction_price < 0 {
                                                "purchased"
                                            } else {
                                                "sold"
                                            },
                                            item_data.name,
                                            transaction_price.abs()
                                        )
                                    };
                                    let mut chatbox_events =
                                        world.resource_mut::<Events<ChatboxEvent>>();
                                    chatbox_events.send(ChatboxEvent::System(message));
                                }
                            }
                        }

                        update_inventory_and_money(world, player_entity, items, Some(money));
                    });
                }
            }
            Ok(ServerMessage::BankOpen) => {
                commands.add(move |world: &mut World| {
                    let mut chatbox_events = world.resource_mut::<Events<BankEvent>>();
                    chatbox_events.send(BankEvent::Show);
                });
            }
            Ok(ServerMessage::BankSetItems { items }) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    let mut slots = vec![None; 160];

                    for (bank_slot_index, item) in items {
                        let bank_slot_index = bank_slot_index as usize;
                        if bank_slot_index > slots.len() {
                            slots.resize(bank_slot_index + 1, None);
                        }
                        slots[bank_slot_index] = item;
                    }

                    commands.add(move |world: &mut World| {
                        world.entity_mut(player_entity).insert(Bank { slots });

                        let mut chatbox_events = world.resource_mut::<Events<BankEvent>>();
                        chatbox_events.send(BankEvent::Show);
                    });
                }
            }
            Ok(ServerMessage::BankUpdateItems { items }) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        if let Some(mut bank) = world.entity_mut(player_entity).get_mut::<Bank>() {
                            for (bank_slot_index, item) in items {
                                let bank_slot_index = bank_slot_index as usize;

                                if bank_slot_index > bank.slots.len() {
                                    bank.slots.resize(bank_slot_index + 1, None);
                                }

                                bank.slots[bank_slot_index] = item;
                            }
                        }
                    });
                }
            }
            Ok(ServerMessage::BankTransaction {
                inventory_item_slot,
                inventory_item,
                inventory_money,
                bank_slot,
                bank_item,
            }) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        if let Some(mut inventory) =
                            world.entity_mut(player_entity).get_mut::<Inventory>()
                        {
                            if let Some(item_slot) =
                                inventory.get_item_slot_mut(inventory_item_slot)
                            {
                                *item_slot = inventory_item;
                            }

                            if let Some(inventory_money) = inventory_money {
                                inventory.money = inventory_money;
                            }
                        }

                        if let Some(mut bank) = world.entity_mut(player_entity).get_mut::<Bank>() {
                            if let Some(bank_slot) = bank.slots.get_mut(bank_slot) {
                                *bank_slot = bank_item;
                            }
                        }
                    });
                }
            }
            Ok(ServerMessage::MoveToggle(MoveToggle {
                entity_id,
                move_mode,
                .. // TODO: run_speed
            })) => {
                if let Some(entity) = client_entity_list.get(entity_id) {
                    commands.entity(entity).insert(move_mode);
                }
            }
            Ok(message) => {
                log::warn!("Received unimplemented game server message: {:#?}", message);
            }
            Err(crossbeam_channel::TryRecvError::Disconnected) => {
                break Err(ConnectionError::ConnectionLost.into());
            }
            Err(crossbeam_channel::TryRecvError::Empty) => break Ok(()),
        }
    };

    if let Err(error) = result {
        // TODO: Store error somewhere to display to user
        log::warn!("Game server connection error: {}", error);
        commands.remove_resource::<GameConnection>();
    }
}
