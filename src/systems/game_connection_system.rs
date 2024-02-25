use arrayvec::ArrayVec;
use bevy::{
    ecs::event::Events,
    math::{Quat, Vec3},
    prelude::{
        Commands, ComputedVisibility, DespawnRecursiveExt, Entity, EventWriter, GlobalTransform,
        Mut, NextState, Res, ResMut, State, Transform, Visibility, World,
    },
};

use rose_data::{
    AbilityType, EquipmentItem, Item, ItemReference, ItemSlotBehaviour, ItemType, SkillCooldown,
    StatusEffectType,
};
use rose_game_common::{
    components::{
        AbilityValues, BasicStatType, BasicStats, CharacterInfo, ClanPoints, DroppedItem,
        Equipment, ExperiencePoints, HealthPoints, Hotbar, Inventory, ItemDrop, ItemSlot, Level,
        ManaPoints, Money, MoveMode, MoveSpeed, Npc, QuestState, SkillList, Stamina, StatPoints,
        StatusEffects, StatusEffectsRegen,
    },
    messages::{
        server::{
            ClanCreateError, LearnSkillError, LevelUpSkillError, PartyMemberInfo,
            PartyMemberInfoOffline, PersonalStoreTransactionStatus, PickupItemDropError,
            ServerMessage, SpawnCommandState,
        },
        PartyItemSharing, PartyXpSharing,
    },
};
use rose_network_common::ConnectionError;

use crate::{
    bundles::{ability_values_add_value_exclusive, ability_values_set_value_exclusive},
    components::{
        Bank, Clan, ClanMember, ClanMembership, ClientEntity, ClientEntityName, ClientEntityType,
        CollisionHeightOnly, CollisionPlayer, Command, CommandCastSkillTarget, Cooldowns, Dead,
        FacingDirection, NextCommand, PartyInfo, PartyOwner, PassiveRecoveryTime, PendingDamage,
        PendingDamageList, PendingSkillEffect, PendingSkillEffectList, PendingSkillTarget,
        PendingSkillTargetList, PersonalStore, PlayerCharacter, Position, VisibleStatusEffects,
    },
    events::{
        BankEvent, ChatboxEvent, ClientEntityEvent, GameConnectionEvent, LoadZoneEvent,
        MessageBoxEvent, PartyEvent, PersonalStoreEvent, QuestTriggerEvent, UseItemEvent,
    },
    resources::{AppState, ClientEntityList, GameConnection, GameData, WorldRates, WorldTime},
};

fn to_next_command(
    command_state: &SpawnCommandState,
    client_entity_list: &ClientEntityList,
) -> NextCommand {
    match *command_state {
        SpawnCommandState::Move {
            target_position,
            target_entity_id,
        } => NextCommand::with_move(
            target_position,
            target_entity_id.and_then(|id| client_entity_list.get(id)),
            None,
        ),
        SpawnCommandState::RunAway { target_position } => {
            NextCommand::with_move(target_position, None, Some(MoveMode::Run))
        }
        SpawnCommandState::Attack {
            target_entity_id, ..
        } => {
            if let Some(target_entity) = client_entity_list.get(target_entity_id) {
                NextCommand::with_attack(target_entity)
            } else {
                NextCommand::default()
            }
        }
        SpawnCommandState::Sit => NextCommand::with_sitting(),
        SpawnCommandState::PersonalStore => NextCommand::with_personal_store(),
        SpawnCommandState::Die => NextCommand::with_die(),
        _ => NextCommand::default(),
    }
}

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
    app_state_current: Res<State<AppState>>,
    mut app_state_next: ResMut<NextState<AppState>>,
    mut client_entity_list: ResMut<ClientEntityList>,
    mut chatbox_events: EventWriter<ChatboxEvent>,
    mut game_connection_events: EventWriter<GameConnectionEvent>,
    mut load_zone_events: EventWriter<LoadZoneEvent>,
    mut use_item_events: EventWriter<UseItemEvent>,
    mut client_entity_events: EventWriter<ClientEntityEvent>,
    mut party_events: EventWriter<PartyEvent>,
    mut personal_store_events: EventWriter<PersonalStoreEvent>,
    mut quest_trigger_events: EventWriter<QuestTriggerEvent>,
    mut message_box_events: EventWriter<MessageBoxEvent>,
) {
    let Some(game_connection) = game_connection else {
        return;
    };

    let result: Result<(), anyhow::Error> = loop {
        match game_connection.server_message_rx.try_recv() {
            Ok(ServerMessage::ConnectionRequestSuccess { .. }) =>{
            client_entity_list.clear();
            }
            Ok(ServerMessage::ConnectionRequestError { .. }) =>{
                break Err(ConnectionError::ConnectionLost.into());
            },
            Ok(ServerMessage::CharacterData { data: character_data }) => {
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
                        .spawn(((
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
                        ),
                        (
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
                        ),
                        (
                            Transform::from_xyz(
                                character_data.position.x / 100.0,
                                character_data.position.z / 100.0 + 100.0,
                                -character_data.position.y / 100.0,
                            ),
                            GlobalTransform::default(),
                            Visibility::default(),
                            ComputedVisibility::default(),
                        )))
                        .id()
                );

                // Emit connected event, character select system will be responsible for
                // starting the load of the next zone once its animations have completed
                game_connection_events.send(GameConnectionEvent::Connected(character_data.zone_id));
                client_entity_list.zone_id = Some(character_data.zone_id);
            }
            Ok(ServerMessage::CharacterDataItems { data }) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands
                        .entity(player_entity)
                        .insert((data.inventory, data.equipment));
                }
            }
            Ok(ServerMessage::CharacterDataQuest { quest_state }) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.entity(player_entity).insert(*quest_state);
                }
            }
            Ok(ServerMessage::JoinZone { entity_id, experience_points, team, health_points, mana_points, world_ticks, craft_rate, world_price_rate, item_price_rate, town_price_rate }) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    let mut entity_commands = commands.entity(player_entity);
                    entity_commands.insert((
                        ClientEntity::new(entity_id, ClientEntityType::Character),
                        CollisionPlayer,
                        Command::with_stop(),
                        NextCommand::with_stop(),
                        FacingDirection::default(),
                        experience_points,
                        team,
                        health_points,
                        mana_points,
                    ));

                    if health_points.hp > 0 {
                        entity_commands.remove::<Dead>();
                    } else {
                        entity_commands.insert((Dead, Command::with_die(), NextCommand::default()));
                    }

                    commands.insert_resource(WorldRates {
                        craft_rate,
                        item_price_rate,
                        world_price_rate,
                        town_price_rate,
                    });
                    commands.insert_resource(WorldTime::new(world_ticks));

                    client_entity_list.clear();
                    client_entity_list.add(entity_id, player_entity);
                    client_entity_list.player_entity_id = Some(entity_id);

                    // Transition to in game state if we are not already
                    if !matches!(app_state_current.get(), AppState::Game) {
                        app_state_next.set(AppState::Game);
                    }
                }
            }
            Ok(ServerMessage::SpawnEntityCharacter { data: message  }) => {
                let status_effects = StatusEffects {
                    active: message.status_effects,
                    ..Default::default()
                };
                let next_command = to_next_command(&message.spawn_command_state, &client_entity_list);
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
                    .spawn(((
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
                    ),
                    (
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
                    ),))
                    .id();

                if let Some((skin, title)) = message.personal_store_info {
                    commands
                        .entity(entity)
                        .insert(PersonalStore::new(title, skin as usize));
                }

                if let Some(clan_membership) = message.clan_membership {
                    commands
                        .entity(entity)
                        .insert(ClanMembership {
                            clan_unique_id: clan_membership.clan_unique_id,
                            mark: clan_membership.mark,
                            level: clan_membership.level,
                            name: clan_membership.name,
                            position: clan_membership.position,
                            contribution: ClanPoints(0),
                        });
                }

                client_entity_list.add(message.entity_id, entity);
            }
            Ok(ServerMessage::SpawnEntityNpc {
                entity_id,
                npc,
                direction,
                position,
                team,
                health,
                spawn_command_state,
                move_mode,
                status_effects,
            }) => {
                let status_effects = StatusEffects {
                    active: status_effects,
                    ..Default::default()
                };
                let ability_values = game_data
                    .ability_value_calculator
                    .calculate_npc(npc.id, &status_effects, None, None)
                    .unwrap();
                let move_speed = MoveSpeed::new(ability_values.get_move_speed(&move_mode));
                let level = Level::new(ability_values.get_level() as u32);
                let next_command = to_next_command(&spawn_command_state, &client_entity_list);

                let entity = commands
                    .spawn((
                        (
                        Command::with_stop(),
                        next_command,
                        npc,
                        team,
                        health,
                        move_mode,
                        Position::new(position),
                        ability_values,
                        level,
                        move_speed,
                        status_effects,
                    ), (
                        ClientEntity::new(entity_id, ClientEntityType::Npc),
                        CollisionHeightOnly,
                        FacingDirection::default(),
                        PendingDamageList::default(),
                        PendingSkillEffectList::default(),
                        PendingSkillTargetList::default(),
                        VisibleStatusEffects::default(),
                        Transform::from_xyz(
                            position.x / 100.0,
                            position.z / 100.0 + 10000.0,
                            -position.y / 100.0,
                        )
                        .with_rotation(Quat::from_axis_angle(
                            Vec3::Y,
                            direction.to_radians(),
                        )),
                        GlobalTransform::default(),
                        Visibility::default(),
                        ComputedVisibility::default(),
                    ),
                    ))
                    .id();

                client_entity_list.add(entity_id, entity);
            }
            Ok(ServerMessage::SpawnEntityMonster { entity_id, npc, position, team, health, spawn_command_state, move_mode, status_effects }) => {
                let status_effects = StatusEffects {
                    active: status_effects,
                    ..Default::default()
                };
                let ability_values = game_data
                    .ability_value_calculator
                    .calculate_npc(npc.id, &status_effects, None, None)
                    .unwrap();
                let move_speed = MoveSpeed::new(ability_values.get_move_speed(&move_mode));
                let level = Level::new(ability_values.get_level() as u32);
                let next_command = to_next_command(&spawn_command_state, &client_entity_list);

                let mut equipment = Equipment::new();
                if let Some(npc_data) = game_data.npcs.get_npc(npc.id) {
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
                    .spawn(((
                        Command::with_stop(),
                        next_command,
                        npc,
                        team,
                        health,
                        move_mode,
                        Position::new(position),
                        ability_values,
                        equipment,
                        level,
                        move_speed,
                        status_effects,
                    ),
                    (
                        ClientEntity::new(entity_id, ClientEntityType::Monster),
                        CollisionHeightOnly,
                        FacingDirection::default(),
                        PendingDamageList::default(),
                        PendingSkillEffectList::default(),
                        PendingSkillTargetList::default(),
                        VisibleStatusEffects::default(),
                        Transform::from_xyz(
                            position.x / 100.0,
                            position.z / 100.0 + 10000.0,
                            -position.y / 100.0,
                        ),
                        GlobalTransform::default(),
                        Visibility::default(),
                        ComputedVisibility::default(),
                    ),))
                    .id();

                client_entity_list.add(entity_id, entity);
            }
            Ok(ServerMessage::SpawnEntityItemDrop { entity_id, dropped_item, position, remaining_time: _, owner_entity_id: _ }) => {
                let name = match &dropped_item {
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
                    .spawn((
                        ClientEntityName::new(name),
                        ItemDrop::with_dropped_item(dropped_item),
                        Position::new(position),
                        ClientEntity::new(entity_id, ClientEntityType::ItemDrop),
                        CollisionHeightOnly,
                        Transform::from_xyz(
                            position.x / 100.0,
                            position.z / 100.0 + 10000.0,
                            -position.y / 100.0,
                        ),
                        GlobalTransform::default(),
                        Visibility::default(),
                        ComputedVisibility::default(),
                    ))
                    .id();

                client_entity_list.add(entity_id, entity);
            }
            Ok(ServerMessage::MoveEntity { entity_id, target_entity_id, distance: _, x, y, z, move_mode }) => {
                if let Some(entity) = client_entity_list.get(entity_id) {
                    let target_entity = target_entity_id
                        .and_then(|id| client_entity_list.get(id));

                    commands.entity(entity).insert(NextCommand::with_move(
                        Vec3::new(x, y, z as f32),
                        target_entity,
                        move_mode,
                    ));
                }
            }
            Ok(ServerMessage::AdjustPosition { entity_id, position }) => {
                if let Some(entity) = client_entity_list.get(entity_id) {
                    commands
                        .entity(entity)
                        .insert(NextCommand::with_move(position, None, None));
                }
            }
            Ok(ServerMessage::StopMoveEntity { entity_id, x: _, y: _, z: _ }) => {
                // TODO: Lerp to XYZ ?
                if let Some(entity) = client_entity_list.get(entity_id) {
                    commands.entity(entity).insert(NextCommand::with_stop());
                }
            }
            Ok(ServerMessage::AttackEntity {
                entity_id,
                target_entity_id,
                distance: _,
                x: _,
                y: _,
                z: _,
            }) => {
                if let Some(entity) = client_entity_list.get(entity_id) {
                    if let Some(target_entity) = client_entity_list.get(target_entity_id) {
                        commands
                            .entity(entity)
                            .insert(NextCommand::with_attack(target_entity));
                    }
                }
            }
            Ok(ServerMessage::RemoveEntities { entity_ids }) => {
                for entity_id in entity_ids {
                    if let Some(entity) = client_entity_list.get(entity_id) {
                        client_entity_list.remove(entity_id);
                        commands.entity(entity).despawn_recursive();
                    }
                }
            }
            Ok(ServerMessage::DamageEntity { attacker_entity_id, defender_entity_id, damage, is_killed, is_immediate, from_skill }) => {
                if let Some(defender_entity) = client_entity_list.get(defender_entity_id) {
                    let attacker_entity =  client_entity_list.get(attacker_entity_id);
                    let killed_by_player = is_killed
                        && client_entity_list.player_entity
                            == client_entity_list.get(attacker_entity_id);

                    commands.add(move |world: &mut World| {
                        let mut defender = world.entity_mut(defender_entity);
                        if let Some(mut pending_damage_list) =
                            defender.get_mut::<PendingDamageList>()
                        {
                            pending_damage_list.push(PendingDamage::new(
                                attacker_entity,
                                damage,
                                is_killed,
                                is_immediate,
                                from_skill,
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
            Ok(ServerMessage::Teleport { entity_id: _, zone_id, x, y, run_mode: _, ride_mode: _ }) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    // Update player position
                    commands
                        .entity(player_entity)
                        .insert((
                            Position::new(Vec3::new(x, y, 0.0)),
                            Transform::from_xyz(x / 100.0, 100.0, -y / 100.0),
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
                    load_zone_events.send(LoadZoneEvent::new(zone_id));
                    client_entity_list.zone_id = Some(zone_id);
                }
            }
            Ok(ServerMessage::LocalChat {
                entity_id,
                text,
            }) => {
                if let Some(chat_entity) = client_entity_list.get(entity_id) {
                    commands.add(move |world: &mut World| {
                        if let Some(name) = world.entity(chat_entity).get::<ClientEntityName>() {
                            let name = name.to_string();
                            world
                                .resource_mut::<Events<ChatboxEvent>>()
                                .send(ChatboxEvent::Say(name, text));
                        }
                    });
                }
            }
            Ok(ServerMessage::ShoutChat { name, text }) => {
                chatbox_events.send(ChatboxEvent::Shout(name, text));
            }
            Ok(ServerMessage::Whisper { from, text }) => {
                chatbox_events.send(ChatboxEvent::Whisper(from, text));
            }
            Ok(ServerMessage::AnnounceChat { name, text }) => {
                chatbox_events.send(ChatboxEvent::Announce(name, text));
            }
            Ok(ServerMessage::UpdateAbilityValueAdd { ability_type, value }) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    chatbox_events.send(ChatboxEvent::System(format!(
                        "Ability {:?} has {} by {}.",
                        ability_type,
                        if value < 0 {
                            "decreased"
                        } else {
                            "increased"
                        },
                        value.abs(),
                    )));

                    commands.add(move |world: &mut World| {
                        let mut player = world.entity_mut(player_entity);
                        ability_values_add_value_exclusive(
                            ability_type,
                            value,
                            &mut player,
                        );
                    });
                }
            }
            Ok(ServerMessage::UpdateAbilityValueSet { ability_type, value }) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    chatbox_events.send(ChatboxEvent::System(format!(
                        "Ability {:?} has been changed to {}.",
                        ability_type, value,
                    )));

                    commands.add(move |world: &mut World| {
                        let mut player = world.entity_mut(player_entity);
                        ability_values_set_value_exclusive(
                            ability_type,
                            value,
                            &mut player,
                        );
                    });
                }
            }
            Ok(ServerMessage::UpdateAmmo { entity_id, ammo_index, item }) => {
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
            Ok(ServerMessage::UpdateEquipment { entity_id, equipment_index, item  }) => {
                if let Some(entity) = client_entity_list.get(entity_id) {
                    commands.add(move |world: &mut World| {
                        if let Some(mut equipment) = world.entity_mut(entity).get_mut::<Equipment>()
                        {
                            if let Some(equipped_item) =
                                equipment.equipped_items[equipment_index].as_mut()
                            {
                                if let Some(item) = item {
                                    // Only update visual related data
                                    equipped_item.item = item.item;
                                    equipped_item.has_socket = item.has_socket;
                                    equipped_item.gem = item.gem;
                                    equipped_item.grade = item.grade;
                                } else {
                                    equipment.equipped_items[equipment_index] = None;
                                }
                            } else {
                                equipment.equipped_items[equipment_index] = item;
                            }
                        }
                    });
                }
            }
            Ok(ServerMessage::UpdateVehiclePart { entity_id, vehicle_part_index, item }) => {
                if let Some(entity) = client_entity_list.get(entity_id) {
                    commands.add(move |world: &mut World| {
                        if let Some(mut equipment) = world.entity_mut(entity).get_mut::<Equipment>()
                        {
                            if let Some(equipped_item) =
                                equipment.equipped_vehicle[vehicle_part_index].as_mut()
                            {
                                if let Some(item) = item {
                                    // Only update visual related data
                                    equipped_item.item = item.item;
                                    equipped_item.has_socket = item.has_socket;
                                    equipped_item.gem = item.gem;
                                    equipped_item.grade = item.grade;
                                } else {
                                    equipment.equipped_vehicle[vehicle_part_index] = None;
                                }
                            } else {
                                equipment.equipped_vehicle[vehicle_part_index] = item;
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
                                    if let Some(equipment_item) = equipment.get_equipment_item_mut(index) {
                                        equipment_item.life = life;
                                    }
                                }
                            },
                            ItemSlot::Vehicle(index) => {
                                if let Some(mut equipment) = world.entity_mut(entity).get_mut::<Equipment>() {
                                    if let Some(equipment_item) = equipment.get_vehicle_item_mut(index) {
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
            Ok(ServerMessage::UpdateInventory { items, money }) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        update_inventory_and_money(
                            world,
                            player_entity,
                            items,
                            money,
                        );
                    });
                }
            }
            Ok(ServerMessage::UseInventoryItem { inventory_slot, .. }) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        if let Some(mut inventory) =
                            world.entity_mut(player_entity).get_mut::<Inventory>()
                        {
                            if let Some(item_slot) =
                                inventory.get_item_slot_mut(inventory_slot)
                            {
                                item_slot.try_take_quantity(1);
                            }
                        }
                    });
                }
            }
            Ok(ServerMessage::UpdateMoney { money }) => {
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
            Ok(ServerMessage::UpdateBasicStat { basic_stat_type, value }) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        world.resource_scope(|world, game_data: Mut<GameData>| {
                            let mut stat_point_cost = None;
                            let mut player = world.entity_mut(player_entity);

                            if let Some(mut basic_stats) = player.get_mut::<BasicStats>() {
                                let current_value = basic_stats.get(basic_stat_type);

                                // Calculate stat point cost if this looked like a user requested stat increase
                                if value == current_value + 1 {
                                    if let Some(cost) = game_data
                                        .ability_value_calculator
                                        .calculate_basic_stat_increase_cost(
                                            &basic_stats,
                                            basic_stat_type,
                                        )
                                    {
                                        stat_point_cost = Some(cost);
                                    }
                                }

                                // Update stats
                                match basic_stat_type {
                                    BasicStatType::Strength => basic_stats.strength = value,
                                    BasicStatType::Dexterity => {
                                        basic_stats.dexterity = value
                                    }
                                    BasicStatType::Intelligence => {
                                        basic_stats.intelligence = value
                                    }
                                    BasicStatType::Concentration => {
                                        basic_stats.concentration = value
                                    }
                                    BasicStatType::Charm => basic_stats.charm = value,
                                    BasicStatType::Sense => basic_stats.sense = value,
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
            Ok(ServerMessage::UpdateLevel { entity_id, level, experience_points, stat_points, skill_points }) => {
                if let Some(entity) = client_entity_list.get(entity_id) {
                    client_entity_events.send(ClientEntityEvent::LevelUp(
                        entity,
                        Some(level.level),
                    ));

                    commands.entity(entity).insert((
                        level,
                        experience_points,
                        stat_points,
                        skill_points,
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
                                    &level,
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
            Ok(ServerMessage::LevelUpEntity { entity_id }) => {
                if client_entity_list.player_entity_id == Some(entity_id) {
                    // Ignore, the server erroneously sends this message in addition to ServerMessage::UpdateLevel
                } else if let Some(entity) = client_entity_list.get(entity_id) {
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
            Ok(ServerMessage::UpdateSpeed { entity_id, run_speed, passive_attack_speed: _ }) => {
                // TODO: Use passive_attack_speed ?
                if let Some(entity) = client_entity_list.get(entity_id) {
                    commands
                        .entity(entity)
                        .insert(MoveSpeed::new(run_speed as f32));
                }
            }
            Ok(ServerMessage::UpdateStatusEffects { entity_id, status_effects: update_status_effects, updated_values }) => {
                if let Some(entity) = client_entity_list.get(entity_id) {
                    commands.add(move |world: &mut World| {
                        let mut entity_mut = world.entity_mut(entity);
                        let mut updated_hp = None;
                        let mut updated_mp = None;

                        // Clear StatusEffects for status effects which do not exist in the packet
                        if let Some(mut status_effects) = entity_mut.get_mut::<StatusEffects>() {
                            for (status_effect_type, active) in update_status_effects.iter() {
                                if active.is_some() {
                                    continue;
                                }

                                if status_effects.active[status_effect_type].is_some() {
                                    match status_effect_type {
                                        StatusEffectType::IncreaseHp => {
                                            updated_hp = updated_values.first().cloned();
                                        },
                                        StatusEffectType::IncreaseMp => {
                                            updated_mp = updated_values.last().cloned();
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
                            for (status_effect_type, active) in update_status_effects {
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
            Ok(ServerMessage::UpdateXpStamina { xp, stamina, source_entity_id: _ }) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        let mut player = world.entity_mut(player_entity);

                        if let Some(mut player_stamina) = player.get_mut::<Stamina>() {
                            player_stamina.stamina = stamina;
                        }

                        if let Some(mut experience_points) = player.get_mut::<ExperiencePoints>() {
                            let previous_xp = experience_points.xp;
                            experience_points.xp = xp;

                            if xp > previous_xp {
                                world.resource_mut::<Events<ChatboxEvent>>().send(
                                    ChatboxEvent::System(format!(
                                        "You have earned {} experience points.",
                                        xp - previous_xp
                                    )),
                                );
                            }
                        }
                    });
                }
            }
            Ok(ServerMessage::PickupDropItem { drop_entity_id: _, item_slot, item }) => {
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
            Ok(ServerMessage::PickupDropMoney { drop_entity_id: _, money }) => {
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
            Ok(ServerMessage::PickupDropError { drop_entity_id: _, error }) => match error{
                PickupItemDropError::InventoryFull => {
                    chatbox_events.send(ChatboxEvent::System(
                        "Cannot pickup item, inventory full.".to_string(),
                    ));
                }
                PickupItemDropError::NoPermission => {
                    chatbox_events.send(ChatboxEvent::System(
                        "Cannot pickup item, it does not belong to you.".to_string(),
                    ));
                }
                PickupItemDropError::NotExist => {}
            },
            Ok(ServerMessage::RewardItems { items }) => {
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
            Ok(ServerMessage::RewardMoney { money }) => {
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
            Ok(ServerMessage::QuestDeleteResult {
                success,
                slot,
                quest_id,
            }) => {
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
            Ok(ServerMessage::QuestTriggerResult {
                success,
                trigger_hash,
            }) => {
                if success {
                    quest_trigger_events.send(QuestTriggerEvent::ApplyRewards(trigger_hash));
                }
            }
            Ok(ServerMessage::RunNpcDeathTrigger { npc_id }) => {
                if let Some(npc_data) = game_data.npcs.get_npc(npc_id) {
                    quest_trigger_events.send(QuestTriggerEvent::DoTrigger(
                        npc_data.death_quest_trigger_name.as_str().into(),
                    ));
                }
            }
            Ok(ServerMessage::SetHotbarSlot { slot_index, slot }) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        let mut player = world.entity_mut(player_entity);
                        if let Some(mut hotbar) = player.get_mut::<Hotbar>() {
                            hotbar.set_slot(slot_index, slot);
                        }
                    });
                }
            }
            Ok(ServerMessage::LearnSkillSuccess { skill_slot, skill_id, updated_skill_points }) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        let mut player = world.entity_mut(player_entity);
                        if let Some(mut skill_list) = player.get_mut::<SkillList>() {
                            if let Some(skill_slot) =
                                skill_list.get_slot_mut(skill_slot)
                            {
                                *skill_slot = skill_id;
                            }
                        }
                    });

                    commands
                        .entity(player_entity)
                        .insert(updated_skill_points);
                }
            }
            Ok(ServerMessage::LearnSkillError { error }) => match error {
                LearnSkillError::AlreadyLearnt => chatbox_events.send(ChatboxEvent::System(
                    "Failed to learn skill, you already know it.".to_string(),
                )),
                LearnSkillError::JobRequirement => chatbox_events.send(ChatboxEvent::System(
                    "Failed to learn skill, you do not satisfy the job requirement.".to_string(),
                )),
                LearnSkillError::SkillRequirement => {
                    chatbox_events.send(ChatboxEvent::System(
                        "Failed to learn skill, you do not satisfy the skill requirement."
                            .to_string(),
                    ))
                }
                LearnSkillError::AbilityRequirement => {
                    chatbox_events.send(ChatboxEvent::System(
                        "Failed to learn skill, you do not satisfy the ability requirement."
                            .to_string(),
                    ))
                }
                LearnSkillError::Full => chatbox_events.send(ChatboxEvent::System(
                    "Failed to learn skill, you have too many skills.".to_string(),
                )),
                LearnSkillError::InvalidSkillId => chatbox_events.send(ChatboxEvent::System(
                    "Failed to learn skill, invalid skill.".to_string(),
                )),
                LearnSkillError::SkillPointRequirement => {
                    chatbox_events.send(ChatboxEvent::System(
                        "Failed to learn skill, not enough skill points.".to_string(),
                    ))
                }
            },
            Ok(ServerMessage::LevelUpSkillSuccess { skill_slot, skill_id, skill_points }) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        let mut player = world.entity_mut(player_entity);
                        player.insert(skill_points);

                        if let Some(mut skill_list) = player.get_mut::<SkillList>() {
                            if let Some(skill_slot) = skill_list.get_slot_mut(skill_slot) {
                                *skill_slot = Some(skill_id);
                            }
                        }
                    });
                }
            }
            Ok(ServerMessage::LevelUpSkillError { error, skill_points }) => {
                match error {
                    LevelUpSkillError::Failed => chatbox_events.send(ChatboxEvent::System(
                        "Failed to level up skill.".to_string(),
                    )),
                    LevelUpSkillError::JobRequirement => {
                        chatbox_events.send(ChatboxEvent::System(
                            "Failed to level up skill, you do not satisfy the job requirement."
                                .to_string(),
                        ))
                    }
                    LevelUpSkillError::SkillRequirement => {
                        chatbox_events.send(ChatboxEvent::System(
                            "Failed to level up skill, you do not satisfy the skill requirement."
                                .to_string(),
                        ))
                    }
                    LevelUpSkillError::AbilityRequirement => {
                        chatbox_events.send(ChatboxEvent::System(
                            "Failed to level up skill, you do not satisfy the ability requirement."
                                .to_string(),
                        ))
                    }
                    LevelUpSkillError::MoneyRequirement => {
                        chatbox_events.send(ChatboxEvent::System(
                            "Failed to level up skill, not enough money.".to_string(),
                        ))
                    }
                    LevelUpSkillError::SkillPointRequirement => {
                        chatbox_events.send(ChatboxEvent::System(
                            "Failed to level up skill, not enough skill points.".to_string(),
                        ))
                    }
                }

                if let Some(player_entity) = client_entity_list.player_entity {
                    commands
                        .entity(player_entity)
                        .insert(skill_points);
                }
            }
            Ok(ServerMessage::UseEmote { entity_id, motion_id, is_stop }) => {
                if let Some(entity) = client_entity_list.get(entity_id) {
                    let new_command = NextCommand::with_emote(motion_id, is_stop);
                    commands.entity(entity).insert(new_command);
                }
            }
            Ok(ServerMessage::SitToggle { entity_id }) => {
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
            Ok(ServerMessage::UseItem { entity_id, item }) => {
                if let Some(entity) = client_entity_list.get(entity_id) {
                    use_item_events.send(UseItemEvent { entity, item });
                }
            }
            Ok(ServerMessage::CastSkillSelf { entity_id, skill_id, cast_motion_id }) => {
                if let Some(entity) = client_entity_list.get(entity_id) {
                    commands.entity(entity).insert(NextCommand::with_cast_skill(
                        skill_id,
                        None,
                        cast_motion_id,
                        None,
                        None,
                    ));

                    if client_entity_list.player_entity == Some(entity) {
                        if let Some(skill_data) = game_data.skills.get_skill(skill_id) {
                            match skill_data.cooldown {
                                SkillCooldown::Skill { duration } => {
                                    commands.add(move |world: &mut World| {
                                        let mut character = world.entity_mut(entity);

                                        if let Some(mut cooldowns) = character.get_mut::<Cooldowns>() {
                                            cooldowns.set_skill_cooldown(skill_id, duration);
                                        }
                                    });
                                },
                                SkillCooldown::Group { group, duration } => {
                                    commands.add(move |world: &mut World| {
                                        let mut character = world.entity_mut(entity);

                                        if let Some(mut cooldowns) = character.get_mut::<Cooldowns>() {
                                            cooldowns.set_skill_group_cooldown(group.get(), duration);
                                        }
                                    });
                                },
                            }
                        }
                    }
                }
            }
            Ok(ServerMessage::CastSkillTargetEntity { entity_id, skill_id, target_entity_id, target_distance: _, target_position: _, cast_motion_id }) => {
                if let Some(entity) = client_entity_list.get(entity_id) {
                    if let Some(target_entity) = client_entity_list.get(target_entity_id) {
                        commands.entity(entity).insert(NextCommand::with_cast_skill(
                            skill_id,
                            Some(CommandCastSkillTarget::Entity(target_entity)),
                            cast_motion_id,
                            None,
                            None,
                        ));
                    }

                    if client_entity_list.player_entity == Some(entity) {
                        if let Some(skill_data) = game_data.skills.get_skill(skill_id) {
                            match skill_data.cooldown {
                                SkillCooldown::Skill { duration } => {
                                    commands.add(move |world: &mut World| {
                                        let mut character = world.entity_mut(entity);

                                        if let Some(mut cooldowns) = character.get_mut::<Cooldowns>() {
                                            cooldowns.set_skill_cooldown(skill_id, duration);
                                        }
                                    });
                                },
                                SkillCooldown::Group { group, duration } => {
                                    commands.add(move |world: &mut World| {
                                        let mut character = world.entity_mut(entity);

                                        if let Some(mut cooldowns) = character.get_mut::<Cooldowns>() {
                                            cooldowns.set_skill_group_cooldown(group.get(), duration);
                                        }
                                    });
                                },
                            }
                        }
                    }
                }
            }
            Ok(ServerMessage::CastSkillTargetPosition { entity_id, skill_id, target_position, cast_motion_id }) => {
                if let Some(entity) = client_entity_list.get(entity_id) {
                    commands.entity(entity).insert(NextCommand::with_cast_skill(
                        skill_id,
                        Some(CommandCastSkillTarget::Position(target_position)),
                        cast_motion_id,
                        None,
                        None,
                    ));

                    if client_entity_list.player_entity == Some(entity) {
                        if let Some(skill_data) = game_data.skills.get_skill(skill_id) {
                            match skill_data.cooldown {
                                SkillCooldown::Skill { duration } => {
                                    commands.add(move |world: &mut World| {
                                        let mut character = world.entity_mut(entity);

                                        if let Some(mut cooldowns) = character.get_mut::<Cooldowns>() {
                                            cooldowns.set_skill_cooldown(skill_id, duration);
                                        }
                                    });
                                },
                                SkillCooldown::Group { group, duration } => {
                                    commands.add(move |world: &mut World| {
                                        let mut character = world.entity_mut(entity);

                                        if let Some(mut cooldowns) = character.get_mut::<Cooldowns>() {
                                            cooldowns.set_skill_group_cooldown(group.get(), duration);
                                        }
                                    });
                                },
                            }
                        }
                    }
                }
            }
            Ok(ServerMessage::CancelCastingSkill { entity_id, reason: _ }) => {
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
            Ok(ServerMessage::StartCastingSkill { entity_id: _ }) => {
                // Nah bruv
            }
            Ok(ServerMessage::FinishCastingSkill { entity_id, skill_id }) => {
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
            Ok(ServerMessage::ApplySkillEffect { entity_id, caster_entity_id, caster_intelligence, skill_id, effect_success }) => {
                if let Some(defender_entity) = client_entity_list.get(entity_id) {
                    let caster_entity = client_entity_list.get(caster_entity_id);

                    commands.add(move |world: &mut World| {
                        let mut defender = world.entity_mut(defender_entity);

                        if let Some(mut pending_skill_effect_list) =
                            defender.get_mut::<PendingSkillEffectList>()
                        {
                            pending_skill_effect_list.push(PendingSkillEffect::new(
                                skill_id,
                                caster_entity,
                                caster_intelligence,
                                effect_success,
                            ));
                        }

                        if let Some(caster_entity) = caster_entity {
                            if let Some(mut pending_skill_target_list) = world
                                .entity_mut(caster_entity)
                                .get_mut::<PendingSkillTargetList>()
                            {
                                pending_skill_target_list.push(PendingSkillTarget::new(
                                    skill_id,
                                    defender_entity,
                                ));
                            }
                        }
                    });
                }
            }
            Ok(ServerMessage::NpcStoreTransactionError { error }) => {
                chatbox_events.send(ChatboxEvent::System(format!(
                    "Store transation failed with error {:?}",
                    error
                )));
            }
            Ok(ServerMessage::PartyCreate { entity_id }) => {
                if let Some(inviter_entity) = client_entity_list.get(entity_id) {
                    party_events.send(PartyEvent::InvitedCreate(inviter_entity));
                }
            }
            Ok(ServerMessage::PartyInvite { entity_id }) => {
                if let Some(inviter_entity) = client_entity_list.get(entity_id) {
                    party_events.send(PartyEvent::InvitedJoin(inviter_entity));
                }
            }
            Ok(ServerMessage::PartyAcceptCreate { entity_id }) => {
                if let Some(invited_entity) = client_entity_list.get(entity_id) {
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
            Ok(ServerMessage::PartyAcceptInvite { .. }) => {
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
            Ok(ServerMessage::PartyRejectInvite { reason: _, entity_id }) => {
                if let Some(invited_entity) = client_entity_list.get(entity_id) {
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
            Ok(ServerMessage::PartyChangeOwner { entity_id }) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    let is_player_owner =
                        Some(entity_id) == client_entity_list.player_entity_id;

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
                                        if member_info_online.entity_id == entity_id {
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
            Ok(ServerMessage::PartyMemberList {
                mut members,
                item_sharing,
                xp_sharing,
                ..
            }) => {
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
            Ok(ServerMessage::PartyMemberLeave {
                leaver_character_id,
                owner_character_id,
            }) => {
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
            Ok(ServerMessage::PartyMemberDisconnect { character_id }) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        if let Some(mut party_info) =
                            world.entity_mut(player_entity).get_mut::<PartyInfo>()
                        {
                            if let Some(party_member) = party_info
                                .members
                                .iter_mut()
                                .find(|x| x.get_character_id() == character_id)
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
            Ok(ServerMessage::PartyMemberKicked { character_id }) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        if let Some(mut party_info) =
                            world.entity_mut(player_entity).get_mut::<PartyInfo>()
                        {
                            if let Some(index) = party_info
                                .members
                                .iter()
                                .position(|x| x.get_character_id() == character_id)
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
            Ok(ServerMessage::PartyMemberUpdateInfo { member_info }) => {
                let member_entity = client_entity_list.get(member_info.entity_id);
                let player_entity = client_entity_list.player_entity;

                if member_entity.is_some() || player_entity.is_some() {
                    commands.add(move |world: &mut World| {
                        if let Some(mut member) = member_entity
                            .and_then(|member_entity| world.get_entity_mut(member_entity))
                        {
                            if let Some(mut basic_stats) = member.get_mut::<BasicStats>() {
                                basic_stats.concentration = member_info.concentration;
                            }

                            if let Some(mut health_points) = member.get_mut::<HealthPoints>() {
                                health_points.hp = member_info.health_points.hp;
                            }
                        }

                        if let Some(mut player) = player_entity
                            .and_then(|player_entity| world.get_entity_mut(player_entity))
                        {
                            if let Some(mut party_info) = player.get_mut::<PartyInfo>() {
                                if let Some(party_member) =
                                    party_info.members.iter_mut().find(|x| {
                                        x.get_character_id() == member_info.character_id
                                    })
                                {
                                    *party_member = PartyMemberInfo::Online(member_info);
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
            Ok(ServerMessage::PartyUpdateRules { item_sharing, xp_sharing }) => {
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
            Ok(ServerMessage::UpdateSkillList { skill_list: update_skills }) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        let mut player = world.entity_mut(player_entity);
                        if let Some(mut skill_list) = player.get_mut::<SkillList>() {
                            for update_skill in update_skills {
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
            Ok(ServerMessage::OpenPersonalStore {
                entity_id,
                skin,
                title,
            }) => {
                if let Some(entity) = client_entity_list.get(entity_id) {
                    commands.entity(entity).insert(PersonalStore {
                        title,
                        skin: skin as usize,
                    });
                }
            }
            Ok(ServerMessage::ClosePersonalStore { entity_id }) => {
                if let Some(entity) = client_entity_list.get(entity_id) {
                    commands.entity(entity).remove::<PersonalStore>();
                }
            }
            Ok(ServerMessage::PersonalStoreItemList { sell_items, buy_items  }) => {
                personal_store_events.send(PersonalStoreEvent::SetItemList {
                    sell_items,
                    buy_items,
                });
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
            Ok(ServerMessage::MoveToggle {
                entity_id,
                move_mode,
                .. // TODO: run_speed
            }) => {
                if let Some(entity) = client_entity_list.get(entity_id) {
                    commands.entity(entity).insert(move_mode);
                }
            }
            Ok(ServerMessage::ChangeNpcId { entity_id, npc_id }) => {
                if let Some(entity) = client_entity_list.get(entity_id) {
                    commands.add(move |world: &mut World| {
                        let mut entity_mut = world.entity_mut(entity);
                        if let Some(mut npc) = entity_mut.get_mut::<Npc>() {
                            npc.id = npc_id;
                        }
                    });
                }
            }
            Ok(ServerMessage::ClanInfo { id, mark, level, points, money, name, description, position, contribution, skills }) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.entity(player_entity).insert((
                        Clan {
                            unique_id: id,
                            name: name.clone(),
                            description,
                            mark,
                            money,
                            points,
                            level,
                            members: Vec::new(),
                            skills,
                        },
                        ClanMembership {
                            clan_unique_id: id,
                            mark,
                            level,
                            name,
                            position,
                            contribution,
                        }));
                }
            }
            Ok(ServerMessage::ClanUpdateInfo { id, mark, level, points, money, skills }) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        let mut entity_mut = world.entity_mut(player_entity);
                        if let Some(mut clan) = entity_mut.get_mut::<Clan>() {
                            clan.unique_id = id;
                            clan.mark = mark;
                            clan.level = level;
                            clan.points = points;
                            clan.money = money;
                            clan.skills = skills;
                        }
                    });
                }
            }
            Ok(ServerMessage::CharacterUpdateClan { client_entity_id, id, name, mark, level, position  }) => {
                if let Some(entity) = client_entity_list.get(client_entity_id) {
                    commands.entity(entity).insert(
                        ClanMembership {
                            clan_unique_id: id,
                            mark,
                            level,
                            name,
                            position,
                            contribution: ClanPoints(0),
                        });
                }
            }
            Ok(ServerMessage::ClanMemberConnected { name, channel_id  }) =>  {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        let mut entity_mut = world.entity_mut(player_entity);
                        if let Some(mut clan) = entity_mut.get_mut::<Clan>() {
                            if let Some(member) = clan.find_member_mut(&name) {
                                member.channel_id = Some(channel_id);
                            }
                        }
                    });
                }
            }
            Ok(ServerMessage::ClanMemberDisconnected { name  }) =>  {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        let mut entity_mut = world.entity_mut(player_entity);
                        if let Some(mut clan) = entity_mut.get_mut::<Clan>() {
                            if let Some(member) = clan.find_member_mut(&name) {
                                member.channel_id = None;
                            }
                        }
                    });
                }
            }
            Ok(ServerMessage::ClanCreateError { error }) =>  {
                match error {
                    ClanCreateError::Failed => {
                        message_box_events.send(MessageBoxEvent::Show { message: game_data.client_strings.clan_create_error.into(), modal: false, ok: None, cancel: None });
                    },
                    ClanCreateError::NameExists => {
                        message_box_events.send(MessageBoxEvent::Show { message: game_data.client_strings.clan_create_error_name.into(), modal: false, ok: None, cancel: None });
                    },
                    ClanCreateError::NoPermission => {
                        message_box_events.send(MessageBoxEvent::Show { message: game_data.client_strings.clan_create_error_permission.into(), modal: false, ok: None, cancel: None });
                    },
                    ClanCreateError::UnmetCondition => {
                        message_box_events.send(MessageBoxEvent::Show { message: game_data.client_strings.clan_create_error_condition.into(), modal: false, ok: None, cancel: None });
                    },
                }
            }
            Ok(ServerMessage::ClanMemberList { members }) =>  {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        let mut entity_mut = world.entity_mut(player_entity);
                        if let Some(mut clan) = entity_mut.get_mut::<Clan>() {
                            clan.members.clear();

                            for member in members {
                                clan.members.push(ClanMember {
                                    name: member.name,
                                    position: member.position,
                                    contribution: member.contribution,
                                    level: member.level,
                                    job: member.job,
                                    channel_id: member.channel_id,
                                });
                            }
                        }
                    });
                }
            }
            Ok(ServerMessage::CraftInsertGem { .. }) => {
                log::warn!("Received unimplemented ServerMessage::CraftInsertGem");
            }
            Ok(ServerMessage::CraftInsertGemError { .. }) => {
                log::warn!("Received unimplemented ServerMessage::CraftInsertGemError");
            }
            Ok(ServerMessage::RepairedItemUsingNpc { .. }) => {
                log::warn!("Received unimplemented ServerMessage::RepairedItemUsingNpc");
            }
            Ok(ServerMessage::LogoutSuccess) => {
                log::warn!("Received unimplemented ServerMessage::LogoutSuccess");
            }
            Ok(ServerMessage::LogoutFailed { .. }) => {
                log::warn!("Received unimplemented ServerMessage::LogoutFailed");
            }
            Ok(ServerMessage::ReturnToCharacterSelect) => {
                log::warn!("Received unimplemented ServerMessage::ReturnToCharacterSelect");
            }
            Ok(ServerMessage::LoginError { .. }) |
            Ok(ServerMessage::LoginSuccess { .. }) |
            Ok(ServerMessage::ChannelList { .. }) |
            Ok(ServerMessage::ChannelListError { .. }) |
            Ok(ServerMessage::JoinServerError {.. }) |
            Ok(ServerMessage::JoinServerSuccess { ..}) |
            Ok(ServerMessage::CharacterList { .. }) |
            Ok(ServerMessage::CharacterListAppend { .. }) |
            Ok(ServerMessage::CreateCharacterSuccess { .. }) |
            Ok(ServerMessage::CreateCharacterError { .. }) |
            Ok(ServerMessage::SelectCharacterSuccess { .. }) |
            Ok(ServerMessage::SelectCharacterError { .. }) |
            Ok(ServerMessage::DeleteCharacterStart { .. }) |
            Ok(ServerMessage::DeleteCharacterCancel { .. }) |
            Ok(ServerMessage::DeleteCharacterError { .. }) => {
                // These should only be login / world server packets, not game server
                log::warn!("Received unexpected game server message");
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
