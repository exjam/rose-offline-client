use bevy::{
    math::{Quat, Vec3},
    prelude::{
        BuildChildren, Commands, ComputedVisibility, DespawnRecursiveExt, Entity, EventWriter,
        GlobalTransform, Or, Query, QuerySet, QueryState, Res, ResMut, State, Transform,
        Visibility, With,
    },
};

use rose_game_common::{
    components::{
        BasicStats, CharacterInfo, Equipment, ExperiencePoints, HealthPoints, Inventory, ItemDrop,
        ManaPoints, MoveMode, MoveSpeed, Npc, SkillList, Stamina, StatusEffects,
    },
    messages::server::{CommandState, PickupItemDropContent, PickupItemDropError, ServerMessage},
};
use rose_network_common::ConnectionError;

use crate::{
    components::{
        ClientEntity, ClientEntityType, CollisionRayCastSource, Command, NextCommand,
        PendingDamageList, PlayerCharacter, Position,
    },
    events::{ChatboxEvent, ClientEntityEvent, GameConnectionEvent},
    resources::{AppState, ClientEntityList, GameConnection, GameData},
};

fn get_entity_name(
    entity: Entity,
    game_data: &GameData,
    query_entity_name: &Query<
        (Option<&CharacterInfo>, Option<&Npc>),
        Or<(With<CharacterInfo>, With<Npc>)>,
    >,
) -> Option<String> {
    match query_entity_name.get(entity) {
        Ok((Some(character_info), None)) => {
            return Some(character_info.name.clone());
        }
        Ok((None, Some(npc))) => {
            if let Some(npc_data) = game_data.npcs.get_npc(npc.id) {
                if !npc_data.name.is_empty() {
                    return Some(npc_data.name.clone());
                }
            }
        }
        _ => {}
    }

    None
}

pub fn game_connection_system(
    mut commands: Commands,
    game_connection: Option<Res<GameConnection>>,
    game_data: Res<GameData>,
    mut app_state: ResMut<State<AppState>>,
    mut chatbox_events: EventWriter<ChatboxEvent>,
    mut client_entity_list: ResMut<ClientEntityList>,
    query_entity_name: Query<
        (Option<&CharacterInfo>, Option<&Npc>),
        Or<(With<CharacterInfo>, With<Npc>)>,
    >,
    mut query_pending_damage_list: Query<&mut PendingDamageList>,
    mut query_set_character: QuerySet<(
        QueryState<(&mut Equipment, &mut Inventory)>,
        QueryState<(
            &mut HealthPoints,
            &mut ManaPoints,
            &CharacterInfo,
            &Equipment,
            &BasicStats,
            &SkillList,
            &StatusEffects,
        )>,
    )>,
    mut query_xp_stamina: Query<(&mut ExperiencePoints, &mut Stamina)>,
    mut game_connection_events: EventWriter<GameConnectionEvent>,
    mut client_entity_events: EventWriter<ClientEntityEvent>,
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
                let move_speed = MoveSpeed::new(ability_values.get_run_speed());

                // Spawn character
                client_entity_list.player_entity = Some(
                    commands
                        .spawn_bundle((
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
                            Position::new(character_data.position),
                        ))
                        .insert_bundle((
                            Command::with_stop(),
                            NextCommand::default(),
                            ability_values,
                            status_effects,
                            move_mode,
                            move_speed,
                            PendingDamageList::default(),
                            PlayerCharacter {},
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

                // Load next zone
                game_connection_events
                    .send(GameConnectionEvent::JoiningZone(character_data.zone_id));
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
                    commands
                        .entity(player_entity)
                        .insert_bundle((
                            ClientEntity::new(message.entity_id, ClientEntityType::Character),
                            Command::with_stop(),
                            NextCommand::default(),
                            message.experience_points,
                            message.team,
                            message.health_points,
                            message.mana_points,
                        ))
                        .with_children(|child_builder| {
                            child_builder.spawn_bundle((
                                CollisionRayCastSource {},
                                Transform::default()
                                    .with_translation(Vec3::new(0.0, 1.35, 0.0))
                                    .looking_at(-Vec3::Y, Vec3::X),
                                GlobalTransform::default(),
                            ));
                        });

                    client_entity_list.clear();
                    client_entity_list.add(message.entity_id, player_entity);
                    client_entity_list.player_entity_id = Some(message.entity_id);
                    // TODO: Do something with message.world_ticks

                    // Transition to in game state if we are not already
                    if !matches!(app_state.current(), AppState::Game) {
                        app_state.set(AppState::Game).ok();
                    }

                    game_connection_events.send(GameConnectionEvent::JoinedZone(
                        client_entity_list.zone_id.unwrap(),
                    ));
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

                /*
                TODO:
                pub personal_store_info: Option<(i32, String)>,
                 */

                let entity = commands
                    .spawn_bundle((
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
                        PendingDamageList::default(),
                    ))
                    .insert_bundle((
                        ClientEntity::new(message.entity_id, ClientEntityType::Character),
                        Transform::from_xyz(
                            message.position.x / 100.0,
                            message.position.z / 100.0 + 10000.0,
                            -message.position.y / 100.0,
                        ),
                        GlobalTransform::default(),
                        Visibility::default(),
                        ComputedVisibility::default(),
                    ))
                    .with_children(|child_builder| {
                        child_builder.spawn_bundle((
                            CollisionRayCastSource {},
                            Transform::default()
                                .with_translation(Vec3::new(0.0, 1.35, 0.0))
                                .looking_at(-Vec3::Y, Vec3::X),
                            GlobalTransform::default(),
                        ));
                    })
                    .id();

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
                let move_speed = match message.move_mode {
                    MoveMode::Walk => MoveSpeed::new(ability_values.get_walk_speed()),
                    MoveMode::Run => MoveSpeed::new(ability_values.get_run_speed()),
                    MoveMode::Drive => MoveSpeed::new(ability_values.get_drive_speed()),
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
                        move_speed,
                        status_effects,
                        PendingDamageList::default(),
                    ))
                    .insert_bundle((
                        ClientEntity::new(message.entity_id, ClientEntityType::Npc),
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
                    .with_children(|child_builder| {
                        child_builder.spawn_bundle((
                            CollisionRayCastSource {},
                            Transform::default()
                                .with_translation(Vec3::new(0.0, 1.35, 0.0))
                                .looking_at(-Vec3::Y, Vec3::X),
                            GlobalTransform::default(),
                        ));
                    })
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
                let move_speed = match message.move_mode {
                    MoveMode::Walk => MoveSpeed::new(ability_values.get_walk_speed()),
                    MoveMode::Run => MoveSpeed::new(ability_values.get_run_speed()),
                    MoveMode::Drive => MoveSpeed::new(ability_values.get_drive_speed()),
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
                        move_speed,
                        status_effects,
                        PendingDamageList::default(),
                    ))
                    .insert_bundle((
                        ClientEntity::new(message.entity_id, ClientEntityType::Monster),
                        Transform::from_xyz(
                            message.position.x / 100.0,
                            message.position.z / 100.0 + 10000.0,
                            -message.position.y / 100.0,
                        ),
                        GlobalTransform::default(),
                        Visibility::default(),
                        ComputedVisibility::default(),
                    ))
                    .with_children(|child_builder| {
                        child_builder.spawn_bundle((
                            CollisionRayCastSource {},
                            Transform::default()
                                .with_translation(Vec3::new(0.0, 1.35, 0.0))
                                .looking_at(-Vec3::Y, Vec3::X),
                            GlobalTransform::default(),
                        ));
                    })
                    .id();

                client_entity_list.add(message.entity_id, entity);
            }
            Ok(ServerMessage::SpawnEntityItemDrop(message)) => {
                // TODO: Use message.remaining_time, message.owner_entity_id ?
                let entity = commands
                    .spawn_bundle((
                        ItemDrop::with_dropped_item(message.dropped_item),
                        Position::new(message.position),
                        ClientEntity::new(message.entity_id, ClientEntityType::ItemDrop),
                        Transform::from_xyz(
                            message.position.x / 100.0,
                            message.position.z / 100.0 + 10000.0,
                            -message.position.y / 100.0,
                        ),
                        GlobalTransform::default(),
                        Visibility::default(),
                        ComputedVisibility::default(),
                    ))
                    .with_children(|child_builder| {
                        child_builder.spawn_bundle((
                            CollisionRayCastSource {},
                            Transform::default()
                                .with_translation(Vec3::new(0.0, 1.35, 0.0))
                                .looking_at(-Vec3::Y, Vec3::X),
                            GlobalTransform::default(),
                        ));
                    })
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
            Ok(ServerMessage::StopMoveEntity(message)) => {
                if let Some(entity) = client_entity_list.get(message.entity_id) {
                    // TODO: Apply the stop entity message.xyz ?
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
                    if let Ok(mut pending_damage_list) =
                        query_pending_damage_list.get_mut(defender_entity)
                    {
                        pending_damage_list.add(
                            message.attacker_entity_id,
                            message.damage,
                            message.is_killed,
                        );
                    }
                }

                if message.is_killed
                    && client_entity_list.player_entity
                        == client_entity_list.get(message.attacker_entity_id)
                {
                    if let Some(entity_name) = client_entity_list
                        .get(message.defender_entity_id)
                        .and_then(|entity| get_entity_name(entity, &game_data, &query_entity_name))
                    {
                        chatbox_events.send(ChatboxEvent::System(format!(
                            "You have succeeded in hunting {}",
                            entity_name
                        )));
                    }
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
                        .remove::<ClientEntity>();

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
                    game_connection_events.send(GameConnectionEvent::JoiningZone(message.zone_id));
                    client_entity_list.zone_id = Some(message.zone_id);
                }
            }
            Ok(ServerMessage::LocalChat(message)) => {
                if let Some(entity) = client_entity_list.get(message.entity_id) {
                    if let Some(name) = get_entity_name(entity, &game_data, &query_entity_name) {
                        chatbox_events.send(ChatboxEvent::Say(name, message.text));
                    }
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
            Ok(ServerMessage::UpdateAmmo(entity_id, ammo_index, item)) => {
                if let Some(entity) = client_entity_list.get(entity_id) {
                    if let Ok((mut equipment, _)) = query_set_character.q0().get_mut(entity) {
                        equipment.equipped_ammo[ammo_index] = item;
                    }
                }
            }
            Ok(ServerMessage::UpdateEquipment(message)) => {
                if let Some(entity) = client_entity_list.get(message.entity_id) {
                    if let Ok((mut equipment, _)) = query_set_character.q0().get_mut(entity) {
                        equipment.equipped_items[message.equipment_index] = message.item;
                    }
                }
            }
            Ok(ServerMessage::UpdateInventory(update_items, update_money)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    if let Ok((_, mut inventory)) = query_set_character.q0().get_mut(player_entity)
                    {
                        for (item_slot, item) in update_items {
                            if let Some(item_slot) = inventory.get_item_slot_mut(item_slot) {
                                *item_slot = item;
                            }
                        }

                        if let Some(money) = update_money {
                            inventory.money = money;
                        }
                    }
                }
            }
            Ok(ServerMessage::UpdateVehiclePart(message)) => {
                if let Some(entity) = client_entity_list.get(message.entity_id) {
                    if let Ok((mut equipment, _)) = query_set_character.q0().get_mut(entity) {
                        equipment.equipped_vehicle[message.vehicle_part_index] = message.item;
                    }
                }
            }
            Ok(ServerMessage::UpdateLevel(message)) => {
                if let Some(entity) = client_entity_list.get(message.entity_id) {
                    commands.entity(entity).insert_bundle((
                        message.level.clone(),
                        message.experience_points,
                        message.stat_points,
                        message.skill_points,
                    ));

                    // Update hp / mp to new max
                    if let Ok((
                        mut health_points,
                        mut mana_points,
                        character_info,
                        equipment,
                        basic_stats,
                        skill_list,
                        status_effects,
                    )) = query_set_character.q1().get_mut(entity)
                    {
                        let ability_values = game_data.ability_value_calculator.calculate(
                            character_info,
                            &message.level,
                            equipment,
                            basic_stats,
                            skill_list,
                            status_effects,
                        );

                        health_points.hp = ability_values.get_max_health();
                        mana_points.mp = ability_values.get_max_mana();
                    }

                    client_entity_events.send(ClientEntityEvent::LevelUp(
                        message.entity_id,
                        message.level.level,
                    ));
                }
            }
            Ok(ServerMessage::UpdateSpeed(message)) => {
                if let Some(entity) = client_entity_list.get(message.entity_id) {
                    commands
                        .entity(entity)
                        .insert(MoveSpeed::new(message.run_speed as f32));
                }
            }
            Ok(ServerMessage::UpdateXpStamina(message)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    if let Ok((mut experience_points, mut stamina)) =
                        query_xp_stamina.get_mut(player_entity)
                    {
                        if message.xp > experience_points.xp {
                            chatbox_events.send(ChatboxEvent::System(format!(
                                "You have earned {} experience points.",
                                message.xp - experience_points.xp
                            )));
                        }

                        experience_points.xp = message.xp;
                        stamina.stamina = message.stamina;
                    }
                }
            }
            Ok(ServerMessage::PickupItemDropResult(message)) => match message.result {
                Ok(PickupItemDropContent::Item(item_slot, item)) => {
                    if let Some(player_entity) = client_entity_list.player_entity {
                        if let Ok((_, mut inventory)) =
                            query_set_character.q0().get_mut(player_entity)
                        {
                            if let Some(inventory_slot) = inventory.get_item_slot_mut(item_slot) {
                                if let Some(item_data) =
                                    game_data.items.get_base_item(item.get_item_reference())
                                {
                                    chatbox_events.send(ChatboxEvent::System(format!(
                                        "You have earned {}.",
                                        item_data.name
                                    )));
                                }
                                *inventory_slot = Some(item);
                            }
                        }
                    }
                }
                Ok(PickupItemDropContent::Money(money)) => {
                    if let Some(player_entity) = client_entity_list.player_entity {
                        if let Ok((_, mut inventory)) =
                            query_set_character.q0().get_mut(player_entity)
                        {
                            chatbox_events.send(ChatboxEvent::System(format!(
                                "You have earned {} zuly.",
                                money.0
                            )));
                            inventory.try_add_money(money).ok();
                        }
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
            Ok(message) => {
                log::warn!("Received unexpected game server message: {:#?}", message);
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
