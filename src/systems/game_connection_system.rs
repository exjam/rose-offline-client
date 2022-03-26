use bevy::{
    math::{Quat, Vec3},
    prelude::{
        BuildChildren, Commands, ComputedVisibility, DespawnRecursiveExt, Entity, EventWriter,
        GlobalTransform, Local, Or, Query, Res, ResMut, State, Transform, Visibility, With,
    },
};

use rose_data::ZoneId;
use rose_game_common::{
    components::{
        CharacterInfo, ExperiencePoints, HealthPoints, MoveMode, MoveSpeed, Npc, Stamina,
        StatusEffects,
    },
    messages::server::{CommandState, ServerMessage},
};
use rose_network_common::ConnectionError;

use crate::{
    components::{
        ClientEntity, ClientEntityId, CollisionRayCastSource, Command, NextCommand,
        PlayerCharacter, Position,
    },
    events::{ChatboxEvent, GameConnectionEvent},
    resources::{AppState, GameConnection, GameData},
};

pub struct ClientEntityList {
    pub client_entities: Vec<Option<Entity>>,
    pub player_entity: Option<Entity>,
    pub player_entity_id: Option<ClientEntityId>,
    pub zone_id: Option<ZoneId>,
}

impl Default for ClientEntityList {
    fn default() -> Self {
        Self {
            client_entities: vec![None; 4096],
            player_entity: None,
            player_entity_id: None,
            zone_id: None,
        }
    }
}

impl ClientEntityList {
    pub fn add(&mut self, id: ClientEntityId, entity: Entity) {
        self.client_entities[id.0 as usize] = Some(entity);
    }

    pub fn remove(&mut self, id: ClientEntityId) {
        self.client_entities[id.0 as usize] = None;
    }

    pub fn clear(&mut self) {
        self.client_entities.fill(None);
    }

    pub fn get(&self, id: ClientEntityId) -> Option<Entity> {
        self.client_entities[id.0 as usize]
    }
}

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
    mut client_entity_list: Local<ClientEntityList>,
    query_entity_name: Query<
        (Option<&CharacterInfo>, Option<&Npc>),
        Or<(With<CharacterInfo>, With<Npc>)>,
    >,
    mut query_xp_stamina: Query<(&mut ExperiencePoints, &mut Stamina)>,
    mut query_health_points: Query<&mut HealthPoints>,
    mut game_connection_events: EventWriter<GameConnectionEvent>,
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
                            ClientEntity::new(message.entity_id),
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
                    ))
                    .insert_bundle((
                        ClientEntity::new(message.entity_id),
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
                    ))
                    .insert_bundle((
                        ClientEntity::new(message.entity_id),
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
                    if let Ok(mut health_points) = query_health_points.get_mut(defender_entity) {
                        if health_points.hp < message.damage.amount as i32 {
                            health_points.hp = 0;
                        } else {
                            health_points.hp -= message.damage.amount as i32;
                        }
                    }

                    if message.is_killed {
                        commands
                            .entity(defender_entity)
                            .insert(NextCommand::with_die())
                            .remove::<ClientEntity>();
                        client_entity_list.remove(message.defender_entity_id);
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
            Ok(ServerMessage::UpdateLevel(message)) => {
                if let Some(entity) = client_entity_list.get(message.entity_id) {
                    commands.entity(entity).insert_bundle((
                        message.level.clone(),
                        message.experience_points,
                        message.stat_points,
                        message.skill_points,
                    ));

                    if Some(entity) == client_entity_list.player_entity {
                        chatbox_events.send(ChatboxEvent::System(format!(
                            "Congratulations! You are now level {}!",
                            message.level.level
                        )));
                    }
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
                                "You have earned {} experience points",
                                message.xp - experience_points.xp
                            )));
                        }

                        experience_points.xp = message.xp;
                        stamina.stamina = message.stamina;
                    }
                }
            }
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
