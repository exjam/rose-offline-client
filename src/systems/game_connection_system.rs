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
        BasicStatType, BasicStats, CharacterInfo, Equipment, ExperiencePoints, HealthPoints,
        Hotbar, Inventory, ItemDrop, ManaPoints, MoveMode, MoveSpeed, Npc, QuestState, SkillList,
        Stamina, StatusEffects,
    },
    messages::server::{
        CommandState, LearnSkillError, LevelUpSkillError, PickupItemDropContent,
        PickupItemDropError, QuestDeleteResult, QuestTriggerResult, ServerMessage,
    },
};
use rose_network_common::ConnectionError;

use crate::{
    components::{
        ClientEntity, ClientEntityType, CollisionRayCastSource, Command, Cooldowns,
        MovementCollisionEntities, NextCommand, PendingDamageList, PersonalStore, PlayerCharacter,
        Position,
    },
    events::{ChatboxEvent, ClientEntityEvent, GameConnectionEvent, QuestTriggerEvent},
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
    query_entity_name: Query<
        (Option<&CharacterInfo>, Option<&Npc>),
        Or<(With<CharacterInfo>, With<Npc>)>,
    >,
    query_movement_collision_entities: Query<&MovementCollisionEntities>,
    mut query_pending_damage_list: Query<&mut PendingDamageList>,
    mut query_hotbar: Query<&mut Hotbar>,
    mut query_set_character: QuerySet<(
        QueryState<(&mut Equipment, &mut Inventory)>,
        QueryState<(
            &mut HealthPoints,
            &mut ManaPoints,
            &CharacterInfo,
            &Equipment,
            &mut BasicStats,
            &mut SkillList,
            &StatusEffects,
        )>,
    )>,
    mut query_quest_state: Query<&mut QuestState>,
    mut query_xp_stamina: Query<(&mut ExperiencePoints, &mut Stamina)>,
    (mut app_state, mut client_entity_list, game_connection, game_data): (
        ResMut<State<AppState>>,
        ResMut<ClientEntityList>,
        Option<Res<GameConnection>>,
        Res<GameData>,
    ),
    (
        mut chatbox_events,
        mut game_connection_events,
        mut client_entity_events,
        mut quest_trigger_events,
    ): (
        EventWriter<ChatboxEvent>,
        EventWriter<GameConnectionEvent>,
        EventWriter<ClientEntityEvent>,
        EventWriter<QuestTriggerEvent>,
    ),
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
                            Cooldowns::default(),
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
                    let down_ray_cast_source = commands
                        .spawn_bundle((
                            CollisionRayCastSource {},
                            Transform::default()
                                .with_translation(Vec3::new(0.0, 1.35, 0.0))
                                .looking_at(-Vec3::Y, Vec3::X),
                            GlobalTransform::default(),
                        ))
                        .id();

                    commands
                        .entity(player_entity)
                        .insert_bundle((
                            ClientEntity::new(message.entity_id, ClientEntityType::Character),
                            MovementCollisionEntities::new(Some(down_ray_cast_source), None),
                            Command::with_stop(),
                            NextCommand::default(),
                            message.experience_points,
                            message.team,
                            message.health_points,
                            message.mana_points,
                        ))
                        .add_child(down_ray_cast_source);

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
                    // Remove colliders, we do not want to process collision until next zone has loaded
                    if let Ok(movement_collision_entities) =
                        query_movement_collision_entities.get(player_entity)
                    {
                        if let Some(down_ray_cast_source) =
                            movement_collision_entities.down_ray_cast_source
                        {
                            commands.entity(down_ray_cast_source).despawn();
                        }

                        if let Some(forward_ray_cast_source) =
                            movement_collision_entities.forward_ray_cast_source
                        {
                            commands.entity(forward_ray_cast_source).despawn();
                        }
                    }

                    // Update player position
                    commands
                        .entity(player_entity)
                        .insert_bundle((
                            Position::new(Vec3::new(message.x, message.y, 0.0)),
                            Transform::from_xyz(message.x / 100.0, 100.0, -message.y / 100.0),
                        ))
                        .remove::<ClientEntity>()
                        .remove::<MovementCollisionEntities>();

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
            Ok(ServerMessage::UpdateMoney(money)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    if let Ok((_, mut inventory)) = query_set_character.q0().get_mut(player_entity)
                    {
                        inventory.money = money;
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
            Ok(ServerMessage::UpdateBasicStat(message)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    if let Ok((_, _, _, _, mut basic_stats, _, _)) =
                        query_set_character.q1().get_mut(player_entity)
                    {
                        match message.basic_stat_type {
                            BasicStatType::Strength => basic_stats.strength = message.value,
                            BasicStatType::Dexterity => basic_stats.dexterity = message.value,
                            BasicStatType::Intelligence => basic_stats.intelligence = message.value,
                            BasicStatType::Concentration => {
                                basic_stats.concentration = message.value
                            }
                            BasicStatType::Charm => basic_stats.charm = message.value,
                            BasicStatType::Sense => basic_stats.sense = message.value,
                        }
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
                            &basic_stats,
                            &skill_list,
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
            Ok(ServerMessage::UpdateStatusEffects(message)) => {
                if let Some(entity) = client_entity_list.get(message.entity_id) {
                    commands.entity(entity).insert(StatusEffects {
                        active: message.status_effects,
                        ..Default::default()
                    });
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
            Ok(ServerMessage::RewardItems(items)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    if let Ok((_, mut inventory)) = query_set_character.q0().get_mut(player_entity)
                    {
                        for (item_slot, item) in items.into_iter() {
                            if let Some(inventory_slot) = inventory.get_item_slot_mut(item_slot) {
                                if let Some(item_data) = item.as_ref().and_then(|item| {
                                    game_data.items.get_base_item(item.get_item_reference())
                                }) {
                                    chatbox_events.send(ChatboxEvent::System(format!(
                                        "You have earned {}.",
                                        item_data.name
                                    )));
                                }

                                *inventory_slot = item;
                            }
                        }
                    }
                }
            }
            Ok(ServerMessage::RewardMoney(money)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    if let Ok((_, mut inventory)) = query_set_character.q0().get_mut(player_entity)
                    {
                        chatbox_events.send(ChatboxEvent::System(format!(
                            "You have earned {} zuly.",
                            money.0
                        )));
                        inventory.try_add_money(money).ok();
                    }
                }
            }
            Ok(ServerMessage::QuestDeleteResult(QuestDeleteResult {
                success,
                slot,
                quest_id,
            })) => {
                if success {
                    let mut quest_state = query_quest_state.single_mut();
                    if let Some(active_quest) = quest_state.active_quests[slot].as_ref() {
                        if active_quest.quest_id == quest_id {
                            quest_state.active_quests[slot] = None;
                        }
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
                let mut hotbar = query_hotbar.single_mut();
                hotbar.set_slot(slot_index, slot);
            }
            Ok(ServerMessage::LearnSkillResult(result)) => match result {
                Ok(message) => {
                    if let Some(player_entity) = client_entity_list.player_entity {
                        if let Ok((_, _, _, _, _, mut skill_list, _)) =
                            query_set_character.q1().get_mut(player_entity)
                        {
                            if let Some(skill_slot) = skill_list.get_slot_mut(message.skill_slot) {
                                *skill_slot = message.skill_id;
                            }

                            commands
                                .entity(player_entity)
                                .insert_bundle((message.updated_skill_points,));
                        }
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
                            if let Ok((_, _, _, _, _, mut skill_list, _)) =
                                query_set_character.q1().get_mut(player_entity)
                            {
                                if let Some(skill_slot) = skill_list.get_slot_mut(skill_slot) {
                                    *skill_slot = Some(skill_id);
                                }
                            }
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
