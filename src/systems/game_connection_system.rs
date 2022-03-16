use bevy::{
    math::Vec3,
    prelude::{
        BuildChildren, Commands, ComputedVisibility, DespawnRecursiveExt, Entity, GlobalTransform,
        Local, Query, Res, ResMut, State, Transform, Visibility, With,
    },
};
use rose_data::ZoneId;
use rose_game_common::{
    components::{
        ClientEntity, ClientEntityId, ClientEntityType, Destination, StatusEffects, Target,
    },
    messages::{client::ClientMessage, server::ServerMessage},
};
use rose_network_common::ConnectionError;

use crate::{
    components::{CollisionRayCastSource, PlayerCharacter},
    resources::{AppState, GameConnection, LoadedZone},
};

pub struct ClientEntityList {
    pub client_entities: Vec<Option<Entity>>,
}

impl Default for ClientEntityList {
    fn default() -> Self {
        Self {
            client_entities: vec![None; 4096],
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

#[allow(clippy::too_many_arguments)]
pub fn game_connection_system(
    mut commands: Commands,
    game_connection: Option<Res<GameConnection>>,
    mut loaded_zone: ResMut<LoadedZone>,
    mut app_state: ResMut<State<AppState>>,
    query_player: Query<Entity, With<PlayerCharacter>>,
    mut client_entity_list: Local<ClientEntityList>,
) {
    if game_connection.is_none() {
        return;
    }

    let game_connection = game_connection.unwrap();
    let result: Result<(), anyhow::Error> = loop {
        match game_connection.server_message_rx.try_recv() {
            Ok(ServerMessage::ConnectionResponse(response)) => match response {
                Ok(_) => {}
                Err(_) => {
                    break Err(ConnectionError::ConnectionLost.into());
                }
            },
            Ok(ServerMessage::CharacterData(character_data)) => {
                // Load next zone
                loaded_zone.next_zone_id = Some(character_data.position.zone_id);

                // Spawn character
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
                        character_data.position.clone(),
                        StatusEffects::default(),
                    ))
                    .insert_bundle((
                        PlayerCharacter {},
                        Transform::from_xyz(
                            character_data.position.position.x / 100.0,
                            20.0,
                            -character_data.position.position.y / 100.0,
                        ),
                        GlobalTransform::default(),
                        Visibility::default(),
                        ComputedVisibility::default(),
                    ));

                // Tell server we are ready to join the zone
                game_connection
                    .client_message_tx
                    .send(ClientMessage::JoinZoneRequest)
                    .ok();

                // We return immediately so future packets will be able to use query_player
                return;
            }
            Ok(ServerMessage::JoinZone(message)) => {
                let entity = query_player.single();
                commands
                    .entity(entity)
                    .insert_bundle((
                        ClientEntity::new(
                            ClientEntityType::Character,
                            message.entity_id,
                            ZoneId::new(1).unwrap(), // TODO: Is ZoneId important in ClientEntity for client?
                        ),
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
                client_entity_list.add(message.entity_id, entity);
                // TODO: Do something with message.world_ticks

                // Transition to in game state if we are not already
                if !matches!(app_state.current(), AppState::Game) {
                    app_state.set(AppState::Game).ok();
                }
            }
            Ok(ServerMessage::SpawnEntityNpc(message)) => {
                // TODO: Rotate using message.direction
                let entity = commands
                    .spawn_bundle((
                        message.npc,
                        message.team,
                        message.health,
                        message.command,
                        message.move_mode,
                        message.position.clone(),
                        StatusEffects {
                            active: message.status_effects,
                            ..Default::default()
                        },
                    ))
                    .insert_bundle((
                        ClientEntity::new(
                            ClientEntityType::Npc,
                            message.entity_id,
                            ZoneId::new(1).unwrap(), // TODO: Is ZoneId important in ClientEntity for client?
                        ),
                        Transform::from_xyz(
                            message.position.position.x / 100.0,
                            100000.0,
                            -message.position.position.y / 100.0,
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

                if let Some(destination) = message.destination.as_ref() {
                    commands.entity(entity).insert(destination.clone());
                }
                if let Some(target_entity) = message
                    .target_entity_id
                    .and_then(|id| client_entity_list.get(id))
                {
                    commands.entity(entity).insert(Target::new(target_entity));
                }

                client_entity_list.add(message.entity_id, entity);
            }
            Ok(ServerMessage::SpawnEntityMonster(message)) => {
                let entity = commands
                    .spawn_bundle((
                        message.npc,
                        message.team,
                        message.health,
                        message.command,
                        message.move_mode,
                        message.position.clone(),
                        StatusEffects {
                            active: message.status_effects,
                            ..Default::default()
                        },
                    ))
                    .insert_bundle((
                        ClientEntity::new(
                            ClientEntityType::Monster,
                            message.entity_id,
                            ZoneId::new(1).unwrap(), // TODO: Is ZoneId important in ClientEntity for client?
                        ),
                        Transform::from_xyz(
                            message.position.position.x / 100.0,
                            100000.0,
                            -message.position.position.y / 100.0,
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

                if let Some(destination) = message.destination.as_ref() {
                    commands.entity(entity).insert(destination.clone());
                }

                if let Some(target_entity) = message
                    .target_entity_id
                    .and_then(|id| client_entity_list.get(id))
                {
                    commands.entity(entity).insert(Target::new(target_entity));
                }

                client_entity_list.add(message.entity_id, entity);
            }
            Ok(ServerMessage::MoveEntity(message)) => {
                if let Some(entity) = client_entity_list.get(message.entity_id) {
                    commands.entity(entity).insert(Destination::new(Vec3::new(
                        message.x,
                        message.y,
                        message.z as f32,
                    )));
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
