use bevy::{
    ecs::query::WorldQuery,
    math::{Quat, Vec3},
    prelude::{
        BuildChildren, Commands, ComputedVisibility, DespawnRecursiveExt, Entity, EventWriter,
        GlobalTransform, ParamSet, Query, Res, ResMut, State, Transform, Visibility,
    },
};

use rose_data::{EquipmentItem, ItemReference, ItemSlotBehaviour, ItemType};
use rose_game_common::{
    components::{
        AbilityValues, BasicStatType, BasicStats, CharacterInfo, DroppedItem, Equipment,
        ExperiencePoints, HealthPoints, Hotbar, Inventory, ItemDrop, ItemSlot, Level, ManaPoints,
        MoveMode, MoveSpeed, QuestState, SkillList, SkillPoints, Stamina, StatPoints,
        StatusEffects, Team, UnionMembership,
    },
    messages::server::{
        CommandState, LearnSkillError, LevelUpSkillError, PickupItemDropContent,
        PickupItemDropError, QuestDeleteResult, QuestTriggerResult, ServerMessage,
        UpdateAbilityValue,
    },
};
use rose_network_common::ConnectionError;

use crate::{
    bundles::{ability_values_add_value, ability_values_set_value},
    components::{
        ClientEntity, ClientEntityName, ClientEntityType, CollisionRayCastSource, Command,
        CommandCastSkillTarget, Cooldowns, MovementCollisionEntities, NextCommand,
        PassiveRecoveryTime, PendingDamageList, PendingSkillEffectList, PendingSkillTargetList,
        PersonalStore, PlayerCharacter, Position, VisibleStatusEffects,
    },
    events::{ChatboxEvent, ClientEntityEvent, GameConnectionEvent, QuestTriggerEvent},
    resources::{AppState, ClientEntityList, GameConnection, GameData, WorldTime},
};

#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct QueryCharacter<'w> {
    pub basic_stats: &'w BasicStats,
    pub character_info: &'w CharacterInfo,
    pub equipment: &'w mut Equipment,
    pub health_points: &'w mut HealthPoints,
    pub mana_points: &'w mut ManaPoints,
    pub skill_list: &'w SkillList,
    pub status_effects: &'w StatusEffects,
}

#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct QueryPlayer<'w> {
    pub entity: Entity,
    pub ability_values: &'w AbilityValues,
    pub character_info: &'w mut CharacterInfo,
    pub basic_stats: &'w mut BasicStats,
    pub equipment: &'w mut Equipment,
    pub experience_points: &'w mut ExperiencePoints,
    pub health_points: &'w mut HealthPoints,
    pub hotbar: &'w mut Hotbar,
    pub inventory: &'w mut Inventory,
    pub level: &'w mut Level,
    pub mana_points: &'w mut ManaPoints,
    pub quest_state: &'w mut QuestState,
    pub skill_list: &'w mut SkillList,
    pub skill_points: &'w mut SkillPoints,
    pub stamina: &'w mut Stamina,
    pub stat_points: &'w mut StatPoints,
    pub status_effects: &'w StatusEffects,
    pub team: &'w mut Team,
    pub union_membership: &'w mut UnionMembership,
}

pub fn game_connection_system(
    mut commands: Commands,
    query_name: Query<&ClientEntityName>,
    query_movement_collision_entities: Query<&MovementCollisionEntities>,
    mut query_pending_damage_list: Query<&mut PendingDamageList>,
    mut query_pending_skill_effect_list: Query<&mut PendingSkillEffectList>,
    mut query_pending_skill_target_list: Query<&mut PendingSkillTargetList>,
    mut query_set_client_entity: ParamSet<(Query<QueryCharacter>, Query<QueryPlayer>)>,
    mut query_command: Query<(&mut Command, &mut NextCommand)>,
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
                            NextCommand::default(),
                            ability_values,
                            status_effects,
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

                    commands.insert_resource(WorldTime::new(message.world_ticks));

                    client_entity_list.clear();
                    client_entity_list.add(message.entity_id, player_entity);
                    client_entity_list.player_entity_id = Some(message.entity_id);

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
                    ))
                    .insert_bundle((
                        ClientEntity::new(message.entity_id, ClientEntityType::Character),
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
                    _ => NextCommand::default(),
                };
                let name = game_data
                    .npcs
                    .get_npc(message.npc.id)
                    .map(|npc_data| npc_data.name.clone())
                    .unwrap_or_else(|| format!("[NPC {}]", message.npc.id.get()));

                let entity = commands
                    .spawn_bundle((
                        ClientEntityName::new(name),
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
                    _ => NextCommand::default(),
                };
                let mut equipment = Equipment::new();
                let name = if let Some(npc_data) = game_data.npcs.get_npc(message.npc.id) {
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

                    npc_data.name.clone()
                } else {
                    format!("[Monster {}]", message.npc.id.get())
                };

                let entity = commands
                    .spawn_bundle((
                        ClientEntityName::new(name),
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
                let name = match &message.dropped_item {
                    DroppedItem::Item(item) => game_data
                        .items
                        .get_base_item(item.get_item_reference())
                        .map(|item_data| item_data.name.clone())
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
                    let new_command = NextCommand::with_move(
                        Vec3::new(message.x, message.y, message.z as f32),
                        target_entity,
                        message.move_mode,
                    );

                    if let Ok((_, mut next_command)) = query_command.get_mut(entity) {
                        *next_command = new_command;
                    } else {
                        commands.entity(entity).insert(new_command);
                    }
                }
            }
            Ok(ServerMessage::StopMoveEntity(message)) => {
                if let Some(entity) = client_entity_list.get(message.entity_id) {
                    // TODO: Apply the stop entity message.xyz ?
                    let new_command = NextCommand::with_stop();

                    if let Ok((_, mut next_command)) = query_command.get_mut(entity) {
                        *next_command = new_command;
                    } else {
                        commands.entity(entity).insert(new_command);
                    }
                }
            }
            Ok(ServerMessage::AttackEntity(message)) => {
                if let Some(entity) = client_entity_list.get(message.entity_id) {
                    if let Some(target_entity) = client_entity_list.get(message.target_entity_id) {
                        let new_command = NextCommand::with_attack(target_entity);

                        if let Ok((_, mut next_command)) = query_command.get_mut(entity) {
                            *next_command = new_command;
                        } else {
                            commands.entity(entity).insert(new_command);
                        }
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
                            message.is_immediate,
                        );
                    }
                }

                if message.is_killed
                    && client_entity_list.player_entity
                        == client_entity_list.get(message.attacker_entity_id)
                {
                    if let Some(entity_name) = client_entity_list
                        .get(message.defender_entity_id)
                        .and_then(|entity| query_name.get(entity).ok())
                    {
                        chatbox_events.send(ChatboxEvent::System(format!(
                            "You have succeeded in hunting {}",
                            entity_name.name
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
                    if let Ok(name) = query_name.get(entity) {
                        chatbox_events.send(ChatboxEvent::Say(name.name.clone(), message.text));
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
            Ok(ServerMessage::UpdateAbilityValue(message)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    if let Ok(mut player) = query_set_client_entity.p1().get_mut(player_entity) {
                        match message {
                            UpdateAbilityValue::RewardAdd(ability_type, add_value) => {
                                ability_values_add_value(
                                    ability_type,
                                    add_value,
                                    player.ability_values,
                                    &mut player.basic_stats,
                                    &mut player.experience_points,
                                    &mut player.health_points,
                                    &mut player.inventory,
                                    &mut player.level,
                                    &mut player.mana_points,
                                    &mut player.skill_points,
                                    &mut player.stamina,
                                    &mut player.stat_points,
                                    &mut player.union_membership,
                                );

                                chatbox_events.send(ChatboxEvent::System(format!(
                                    "Ability {:?} has increased by {}.",
                                    ability_type, add_value,
                                )));
                            }
                            UpdateAbilityValue::RewardSet(ability_type, set_value) => {
                                ability_values_set_value(
                                    ability_type,
                                    set_value,
                                    player.ability_values,
                                    &mut player.basic_stats,
                                    &mut player.character_info,
                                    &mut player.health_points,
                                    &mut player.mana_points,
                                    &mut player.experience_points,
                                    &mut player.level,
                                    &mut player.team,
                                    &mut player.union_membership,
                                );

                                chatbox_events.send(ChatboxEvent::System(format!(
                                    "Ability {:?} has been changed to {}.",
                                    ability_type, set_value,
                                )));
                            }
                        }
                    }
                }
            }
            Ok(ServerMessage::UpdateAmmo(entity_id, ammo_index, item)) => {
                if let Some(entity) = client_entity_list.get(entity_id) {
                    if let Ok(mut character) = query_set_client_entity.p0().get_mut(entity) {
                        if let Some(equipped_ammo) =
                            character.equipment.equipped_ammo[ammo_index].as_mut()
                        {
                            if let Some(item) = item {
                                equipped_ammo.item = item.item;
                            } else {
                                character.equipment.equipped_ammo[ammo_index] = None;
                            }
                        }
                    }
                }
            }
            Ok(ServerMessage::UpdateEquipment(message)) => {
                if let Some(entity) = client_entity_list.get(message.entity_id) {
                    if let Ok(mut character) = query_set_client_entity.p0().get_mut(entity) {
                        if let Some(equipped_item) =
                            character.equipment.equipped_items[message.equipment_index].as_mut()
                        {
                            if let Some(item) = message.item {
                                // Only update visual related data
                                equipped_item.item = item.item;
                                equipped_item.has_socket = item.has_socket;
                                equipped_item.gem = item.gem;
                                equipped_item.grade = item.grade;
                            } else {
                                character.equipment.equipped_items[message.equipment_index] = None;
                            }
                        } else {
                            character.equipment.equipped_items[message.equipment_index] =
                                message.item;
                        }
                    }
                }
            }
            Ok(ServerMessage::UpdateVehiclePart(message)) => {
                if let Some(entity) = client_entity_list.get(message.entity_id) {
                    if let Ok(mut character) = query_set_client_entity.p0().get_mut(entity) {
                        if let Some(equipped_item) = character.equipment.equipped_vehicle
                            [message.vehicle_part_index]
                            .as_mut()
                        {
                            if let Some(item) = message.item {
                                // Only update visual related data
                                equipped_item.item = item.item;
                                equipped_item.has_socket = item.has_socket;
                                equipped_item.gem = item.gem;
                                equipped_item.grade = item.grade;
                            } else {
                                character.equipment.equipped_vehicle[message.vehicle_part_index] =
                                    None;
                            }
                        } else {
                            character.equipment.equipped_vehicle[message.vehicle_part_index] =
                                message.item;
                        }
                    }
                }
            }
            Ok(ServerMessage::UpdateInventory(update_items, update_money)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    if let Ok(mut player) = query_set_client_entity.p1().get_mut(player_entity) {
                        for (item_slot, item) in update_items {
                            match item_slot {
                                ItemSlot::Inventory(_, _) => {
                                    if let Some(item_slot) =
                                        player.inventory.get_item_slot_mut(item_slot)
                                    {
                                        *item_slot = item;
                                    }
                                }
                                ItemSlot::Ammo(ammo_index) => {
                                    *player.equipment.get_ammo_slot_mut(ammo_index) =
                                        item.and_then(|x| x.as_stackable().cloned())
                                }
                                ItemSlot::Equipment(equipment_index) => {
                                    *player.equipment.get_equipment_slot_mut(equipment_index) =
                                        item.and_then(|x| x.as_equipment().cloned())
                                }
                                ItemSlot::Vehicle(vehicle_part_index) => {
                                    *player.equipment.get_vehicle_slot_mut(vehicle_part_index) =
                                        item.and_then(|x| x.as_equipment().cloned())
                                }
                            }
                        }

                        if let Some(money) = update_money {
                            player.inventory.money = money;
                        }
                    }
                }
            }
            Ok(ServerMessage::UseInventoryItem(message)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    if let Ok(mut player) = query_set_client_entity.p1().get_mut(player_entity) {
                        if let Some(item_slot) =
                            player.inventory.get_item_slot_mut(message.inventory_slot)
                        {
                            item_slot.try_take_quantity(1);
                        }
                    }
                }
            }
            Ok(ServerMessage::UpdateMoney(money)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    if let Ok(mut player) = query_set_client_entity.p1().get_mut(player_entity) {
                        player.inventory.money = money;
                    }
                }
            }
            Ok(ServerMessage::UpdateBasicStat(message)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    if let Ok(mut player) = query_set_client_entity.p1().get_mut(player_entity) {
                        // Update stat points if this was a user requested stat increase
                        let current_value = player.basic_stats.get(message.basic_stat_type);
                        if message.value == current_value + 1 {
                            if let Some(cost) = game_data
                                .ability_value_calculator
                                .calculate_basic_stat_increase_cost(
                                    &player.basic_stats,
                                    message.basic_stat_type,
                                )
                            {
                                player.stat_points.points -= player.stat_points.points.min(cost);
                            }
                        }

                        // Update stats
                        match message.basic_stat_type {
                            BasicStatType::Strength => player.basic_stats.strength = message.value,
                            BasicStatType::Dexterity => {
                                player.basic_stats.dexterity = message.value
                            }
                            BasicStatType::Intelligence => {
                                player.basic_stats.intelligence = message.value
                            }
                            BasicStatType::Concentration => {
                                player.basic_stats.concentration = message.value
                            }
                            BasicStatType::Charm => player.basic_stats.charm = message.value,
                            BasicStatType::Sense => player.basic_stats.sense = message.value,
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
                    if let Ok(mut character) = query_set_client_entity.p0().get_mut(entity) {
                        let ability_values = game_data.ability_value_calculator.calculate(
                            character.character_info,
                            &message.level,
                            &character.equipment,
                            character.basic_stats,
                            character.skill_list,
                            character.status_effects,
                        );

                        character.health_points.hp = ability_values.get_max_health();
                        character.mana_points.mp = ability_values.get_max_mana();
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
                    if let Ok(mut player) = query_set_client_entity.p1().get_mut(player_entity) {
                        if message.xp > player.experience_points.xp {
                            chatbox_events.send(ChatboxEvent::System(format!(
                                "You have earned {} experience points.",
                                message.xp - player.experience_points.xp
                            )));
                        }

                        player.experience_points.xp = message.xp;
                        player.stamina.stamina = message.stamina;
                    }
                }
            }
            Ok(ServerMessage::PickupItemDropResult(message)) => match message.result {
                Ok(PickupItemDropContent::Item(item_slot, item)) => {
                    if let Some(player_entity) = client_entity_list.player_entity {
                        if let Ok(mut player) = query_set_client_entity.p1().get_mut(player_entity)
                        {
                            if let Some(inventory_slot) =
                                player.inventory.get_item_slot_mut(item_slot)
                            {
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
                        if let Ok(mut player) = query_set_client_entity.p1().get_mut(player_entity)
                        {
                            chatbox_events.send(ChatboxEvent::System(format!(
                                "You have earned {} zuly.",
                                money.0
                            )));
                            player.inventory.try_add_money(money).ok();
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
                    if let Ok(mut player) = query_set_client_entity.p1().get_mut(player_entity) {
                        for (item_slot, item) in items.into_iter() {
                            if let Some(inventory_slot) =
                                player.inventory.get_item_slot_mut(item_slot)
                            {
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
                    if let Ok(mut player) = query_set_client_entity.p1().get_mut(player_entity) {
                        chatbox_events.send(ChatboxEvent::System(format!(
                            "You have earned {} zuly.",
                            money.0
                        )));
                        player.inventory.try_add_money(money).ok();
                    }
                }
            }
            Ok(ServerMessage::QuestDeleteResult(QuestDeleteResult {
                success,
                slot,
                quest_id,
            })) => {
                if success {
                    if let Some(player_entity) = client_entity_list.player_entity {
                        if let Ok(mut player) = query_set_client_entity.p1().get_mut(player_entity)
                        {
                            if let Some(active_quest) =
                                player.quest_state.active_quests[slot].as_ref()
                            {
                                if active_quest.quest_id == quest_id {
                                    player.quest_state.active_quests[slot] = None;
                                }
                            }
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
                if let Some(player_entity) = client_entity_list.player_entity {
                    if let Ok(mut player) = query_set_client_entity.p1().get_mut(player_entity) {
                        player.hotbar.set_slot(slot_index, slot);
                    }
                }
            }
            Ok(ServerMessage::LearnSkillResult(result)) => match result {
                Ok(message) => {
                    if let Some(player_entity) = client_entity_list.player_entity {
                        if let Ok(mut player) = query_set_client_entity.p1().get_mut(player_entity)
                        {
                            if let Some(skill_slot) =
                                player.skill_list.get_slot_mut(message.skill_slot)
                            {
                                *skill_slot = message.skill_id;
                            }

                            commands
                                .entity(player.entity)
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
                            if let Ok(mut player) =
                                query_set_client_entity.p1().get_mut(player_entity)
                            {
                                if let Some(skill_slot) = player.skill_list.get_slot_mut(skill_slot)
                                {
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
            Ok(ServerMessage::UseEmote(message)) => {
                if let Some(entity) = client_entity_list.get(message.entity_id) {
                    let new_command = NextCommand::with_emote(message.motion_id, message.is_stop);

                    if let Ok((_, mut next_command)) = query_command.get_mut(entity) {
                        *next_command = new_command;
                    } else {
                        commands.entity(entity).insert(new_command);
                    }
                }
            }
            Ok(ServerMessage::SitToggle(entity_id)) => {
                if let Some(entity) = client_entity_list.get(entity_id) {
                    if let Ok((command, mut next_command)) = query_command.get_mut(entity) {
                        if matches!(*command, Command::Sit(_)) {
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
                }
            }
            Ok(ServerMessage::UseItem(message)) => {
                client_entity_events
                    .send(ClientEntityEvent::UseItem(message.entity_id, message.item));
            }
            Ok(ServerMessage::CastSkillSelf(message)) => {
                if let Some(entity) = client_entity_list.get(message.entity_id) {
                    let new_command = NextCommand::with_cast_skill(
                        message.skill_id,
                        None,
                        message.cast_motion_id,
                        None,
                        None,
                    );

                    if let Ok((_, mut next_command)) = query_command.get_mut(entity) {
                        *next_command = new_command;
                    } else {
                        commands.entity(entity).insert(new_command);
                    }
                }
            }
            Ok(ServerMessage::CastSkillTargetEntity(message)) => {
                if let Some(entity) = client_entity_list.get(message.entity_id) {
                    if let Some(target_entity) = client_entity_list.get(message.target_entity_id) {
                        let new_command = NextCommand::with_cast_skill(
                            message.skill_id,
                            Some(CommandCastSkillTarget::Entity(target_entity)),
                            message.cast_motion_id,
                            None,
                            None,
                        );

                        if let Ok((_, mut next_command)) = query_command.get_mut(entity) {
                            *next_command = new_command;
                        } else {
                            commands.entity(entity).insert(new_command);
                        }
                    }
                }
            }
            Ok(ServerMessage::CastSkillTargetPosition(message)) => {
                if let Some(entity) = client_entity_list.get(message.entity_id) {
                    let new_command = NextCommand::with_cast_skill(
                        message.skill_id,
                        Some(CommandCastSkillTarget::Position(message.target_position)),
                        message.cast_motion_id,
                        None,
                        None,
                    );

                    if let Ok((_, mut next_command)) = query_command.get_mut(entity) {
                        *next_command = new_command;
                    } else {
                        commands.entity(entity).insert(new_command);
                    }
                }
            }
            Ok(ServerMessage::CancelCastingSkill(entity_id, _)) => {
                if let Some(entity) = client_entity_list.get(entity_id) {
                    if let Ok((mut command, _)) = query_command.get_mut(entity) {
                        if let Command::CastSkill(_) = command.as_mut() {
                            *command = Command::with_stop();
                        }
                    }
                }
            }
            Ok(ServerMessage::StartCastingSkill(_entity_id)) => {
                // Nah bruv
            }
            Ok(ServerMessage::FinishCastingSkill(entity_id, skill_id)) => {
                if let Some(entity) = client_entity_list.get(entity_id) {
                    if let Ok((mut command, _)) = query_command.get_mut(entity) {
                        if let Command::CastSkill(command_cast_skill) = command.as_mut() {
                            if command_cast_skill.skill_id == skill_id {
                                command_cast_skill.ready_action = true;
                            }
                        }
                    }
                }
            }
            Ok(ServerMessage::ApplySkillEffect(message)) => {
                if let Some(defender_entity) = client_entity_list.get(message.entity_id) {
                    let caster_entity = client_entity_list.get(message.caster_entity_id);

                    if let Ok(mut pending_skill_effect_list) =
                        query_pending_skill_effect_list.get_mut(defender_entity)
                    {
                        pending_skill_effect_list.add_effect(
                            message.skill_id,
                            caster_entity,
                            message.caster_intelligence,
                            message.effect_success,
                        );
                    }

                    if let Some(caster_entity) = caster_entity {
                        if let Ok(mut pending_skill_target_list) =
                            query_pending_skill_target_list.get_mut(caster_entity)
                        {
                            pending_skill_target_list.add_target(message.skill_id, defender_entity);
                        }
                    }
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
