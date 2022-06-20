use bevy::{
    ecs::{event::Events, query::WorldQuery},
    math::{Quat, Vec3},
    prelude::{
        BuildChildren, Commands, ComputedVisibility, DespawnRecursiveExt, Entity, EventWriter,
        GlobalTransform, Mut, Res, ResMut, State, Transform, Visibility, World,
    },
};

use rose_data::{AbilityType, EquipmentItem, ItemReference, ItemSlotBehaviour, ItemType};
use rose_game_common::{
    components::{
        AbilityValues, BasicStatType, BasicStats, CharacterInfo, DroppedItem, Equipment,
        ExperiencePoints, HealthPoints, Hotbar, Inventory, ItemDrop, ItemSlot, Level, ManaPoints,
        MoveMode, MoveSpeed, QuestState, SkillList, SkillPoints, Stamina, StatPoints,
        StatusEffects, Team, UnionMembership,
    },
    messages::server::{
        CommandState, LearnSkillError, LevelUpSkillError, PartyMemberInfo, PartyMemberInfoOffline,
        PartyMemberLeave, PartyMemberList, PickupItemDropContent, PickupItemDropError,
        QuestDeleteResult, QuestTriggerResult, ServerMessage, UpdateAbilityValue,
    },
};
use rose_network_common::ConnectionError;

use crate::{
    bundles::{ability_values_add_value_exclusive, ability_values_set_value_exclusive},
    components::{
        ClientEntity, ClientEntityName, ClientEntityType, CollisionRayCastSource, Command,
        CommandCastSkillTarget, Cooldowns, MovementCollisionEntities, NextCommand, PartyInfo,
        PartyMembership, PartyOwner, PassiveRecoveryTime, PendingDamage, PendingDamageList,
        PendingSkillEffect, PendingSkillEffectList, PendingSkillTarget, PendingSkillTargetList,
        PersonalStore, PlayerCharacter, Position, VisibleStatusEffects,
    },
    events::{ChatboxEvent, ClientEntityEvent, GameConnectionEvent, PartyEvent, QuestTriggerEvent},
    resources::{AppState, ClientEntityList, GameConnection, GameData, WorldRates, WorldTime},
};

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
    pub party_membership: &'w mut PartyMembership,
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
    game_connection: Option<Res<GameConnection>>,
    game_data: Res<GameData>,
    mut app_state: ResMut<State<AppState>>,
    mut client_entity_list: ResMut<ClientEntityList>,
    mut chatbox_events: EventWriter<ChatboxEvent>,
    mut game_connection_events: EventWriter<GameConnectionEvent>,
    mut client_entity_events: EventWriter<ClientEntityEvent>,
    mut party_events: EventWriter<PartyEvent>,
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
                            PartyMembership::default(),
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
                    let killed_by_player = client_entity_list.player_entity
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
                    // Remove colliders, we do not want to process collision until next zone has loaded
                    commands.add(move |world: &mut World| {
                        if let Some(movement_collision_entities) = world
                            .entity(player_entity)
                            .get::<MovementCollisionEntities>(
                        ) {
                            let down_ray_cast_source =
                                movement_collision_entities.down_ray_cast_source;
                            let forward_ray_cast_source =
                                movement_collision_entities.forward_ray_cast_source;

                            if let Some(down_ray_cast_source) = down_ray_cast_source {
                                world.despawn(down_ray_cast_source);
                            }

                            if let Some(forward_ray_cast_source) = forward_ray_cast_source {
                                world.despawn(forward_ray_cast_source);
                            }
                        }
                    });

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
            Ok(ServerMessage::UpdateInventory(update_items, update_money)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        let mut player = world.entity_mut(player_entity);

                        if let Some(mut inventory) = player.get_mut::<Inventory>() {
                            for (item_slot, item) in update_items.iter() {
                                if let ItemSlot::Inventory(_, _) = item_slot {
                                    if let Some(item_slot) = inventory.get_item_slot_mut(*item_slot)
                                    {
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
                        message.entity_id,
                        message.level.level,
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
                client_entity_events
                    .send(ClientEntityEvent::UseItem(message.entity_id, message.item));
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
                                }
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
            Ok(ServerMessage::PartyAcceptCreate(_)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        if let Some(mut party_membership) =
                            world.entity_mut(player_entity).get_mut::<PartyMembership>()
                        {
                            *party_membership = PartyMembership::Member(PartyInfo {
                                owner: PartyOwner::Player,
                                ..Default::default()
                            });
                        }
                    });
                }
            }
            Ok(ServerMessage::PartyAcceptInvite(_)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        if let Some(mut party_membership) =
                            world.entity_mut(player_entity).get_mut::<PartyMembership>()
                        {
                            *party_membership = PartyMembership::Member(PartyInfo::default());
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
                        if let Some(mut party_membership) =
                            world.entity_mut(player_entity).get_mut::<PartyMembership>()
                        {
                            if let PartyMembership::Member(ref mut party_info) =
                                &mut *party_membership
                            {
                                if is_player_owner {
                                    party_info.owner = PartyOwner::Player;
                                } else {
                                    party_info.owner = PartyOwner::Unknown;

                                    for member in party_info.members.iter() {
                                        if let PartyMemberInfo::Online(member_info_online) = member
                                        {
                                            if member_info_online.entity_id == client_entity_id {
                                                party_info.owner = PartyOwner::Character(
                                                    member_info_online.character_id,
                                                );
                                                break;
                                            }
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
                    commands.entity(player_entity).insert(PartyMembership::None);
                }
            }
            Ok(ServerMessage::PartyMemberList(PartyMemberList { mut members, .. })) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        if let Some(mut party_membership) =
                            world.entity_mut(player_entity).get_mut::<PartyMembership>()
                        {
                            if party_membership.is_none() {
                                *party_membership = PartyMembership::Member(PartyInfo::default());
                            }

                            if let PartyMembership::Member(ref mut party_info) =
                                &mut *party_membership
                            {
                                if matches!(party_info.owner, PartyOwner::Unknown) {
                                    party_info.owner =
                                        PartyOwner::Character(members[0].get_character_id());
                                }

                                party_info.members.append(&mut members);
                            }
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

                        if let Some(mut party_membership) = player.get_mut::<PartyMembership>() {
                            if let PartyMembership::Member(ref mut party_info) =
                                &mut *party_membership
                            {
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
                                    party_info.members.remove(index);
                                }
                            }
                        }
                    });
                }
            }
            Ok(ServerMessage::PartyMemberDisconnect(character_unique_id)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        if let Some(mut party_membership) =
                            world.entity_mut(player_entity).get_mut::<PartyMembership>()
                        {
                            if let PartyMembership::Member(ref mut party_info) =
                                &mut *party_membership
                            {
                                if let Some(party_member) = party_info
                                    .members
                                    .iter_mut()
                                    .find(|x| x.get_character_id() == character_unique_id)
                                {
                                    if let PartyMemberInfo::Online(party_member_online) =
                                        party_member
                                    {
                                        *party_member =
                                            PartyMemberInfo::Offline(PartyMemberInfoOffline {
                                                character_id: party_member_online.character_id,
                                                name: party_member_online.name.clone(),
                                            });
                                    }
                                }
                            }
                        }
                    });
                }
            }
            Ok(ServerMessage::PartyMemberKicked(character_unique_id)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        if let Some(mut party_membership) =
                            world.entity_mut(player_entity).get_mut::<PartyMembership>()
                        {
                            if let PartyMembership::Member(ref mut party_info) =
                                &mut *party_membership
                            {
                                if let Some(index) = party_info
                                    .members
                                    .iter()
                                    .position(|x| x.get_character_id() == character_unique_id)
                                {
                                    party_info.members.remove(index);
                                }
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
                            if let Some(mut party_membership) = player.get_mut::<PartyMembership>()
                            {
                                if let PartyMembership::Member(ref mut party_info) =
                                    &mut *party_membership
                                {
                                    if let Some(party_member) =
                                        party_info.members.iter_mut().find(|x| {
                                            x.get_character_id() == party_member_info.character_id
                                        })
                                    {
                                        *party_member = PartyMemberInfo::Online(party_member_info);
                                    }
                                }
                            }
                        }
                    });
                }
            }
            Ok(ServerMessage::PartyUpdateRules(item_sharing, xp_sharing)) => {
                if let Some(player_entity) = client_entity_list.player_entity {
                    commands.add(move |world: &mut World| {
                        if let Some(mut party_membership) =
                            world.entity_mut(player_entity).get_mut::<PartyMembership>()
                        {
                            if let PartyMembership::Member(ref mut party_info) =
                                &mut *party_membership
                            {
                                party_info.item_sharing = item_sharing;
                                party_info.xp_sharing = xp_sharing;
                            }
                        }
                    });
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
