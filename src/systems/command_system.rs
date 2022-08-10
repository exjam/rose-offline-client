use bevy::{
    ecs::system::EntityCommands,
    hierarchy::DespawnRecursiveExt,
    math::{Quat, Vec3, Vec3Swizzles},
    prelude::{AssetServer, Commands, Entity, EventWriter, Handle, Mut, Query, Res, Transform},
};
use rand::prelude::SliceRandom;

use rose_data::{CharacterMotionAction, EquipmentIndex, NpcMotionAction, SkillActionMode};
use rose_file_readers::VfsPathBuf;
use rose_game_common::{
    components::{
        AbilityValues, CharacterGender, Destination, Equipment, HealthPoints, MoveMode, MoveSpeed,
        Npc, Target,
    },
    messages::client::ClientMessage,
};

use crate::{
    components::{
        ActiveMotion, CharacterModel, ClientEntity, ClientEntityType, Command, CommandAttack,
        CommandCastSkill, CommandCastSkillState, CommandCastSkillTarget, CommandEmote, CommandMove,
        CommandSit, NextCommand, NpcModel, PersonalStore, PlayerCharacter, Position,
    },
    events::{ClientEntityEvent, ConversationDialogEvent, PersonalStoreEvent},
    resources::{GameConnection, GameData},
    zmo_asset_loader::ZmoAsset,
};

const NPC_MOVE_TO_DISTANCE: f32 = 250.0;
const CHARACTER_MOVE_TO_DISTANCE: f32 = 1000.0;
const ITEM_DROP_MOVE_TO_DISTANCE: f32 = 150.0;

fn get_attack_animation<R: rand::Rng + ?Sized>(
    rng: &mut R,
    character_model: Option<&CharacterModel>,
    npc_model: Option<&NpcModel>,
) -> Option<Handle<ZmoAsset>> {
    if let Some(character_model) = character_model {
        let mut action = *[
            CharacterMotionAction::Attack,
            CharacterMotionAction::Attack2,
            CharacterMotionAction::Attack3,
        ]
        .choose(rng)
        .unwrap();

        if !character_model.action_motions[action].is_strong() {
            // Not all weapons have 3 attack animations ?
            action = CharacterMotionAction::Attack;
        }

        if character_model.action_motions[action].is_strong() {
            Some(character_model.action_motions[action].clone())
        } else {
            None
        }
    } else if let Some(npc_model) = npc_model {
        if npc_model.action_motions[NpcMotionAction::Attack].is_strong() {
            Some(npc_model.action_motions[NpcMotionAction::Attack].clone())
        } else {
            None
        }
    } else {
        None
    }
}

fn get_die_animation(
    character_model: Option<&CharacterModel>,
    npc_model: Option<&NpcModel>,
) -> Option<Handle<ZmoAsset>> {
    if let Some(character_model) = character_model {
        if character_model.action_motions[CharacterMotionAction::Die].is_strong() {
            Some(character_model.action_motions[CharacterMotionAction::Die].clone())
        } else {
            None
        }
    } else if let Some(npc_model) = npc_model {
        if npc_model.action_motions[NpcMotionAction::Die].is_strong() {
            Some(npc_model.action_motions[NpcMotionAction::Die].clone())
        } else {
            None
        }
    } else {
        None
    }
}

fn get_move_animation(
    move_mode: &MoveMode,
    character_model: Option<&CharacterModel>,
    npc_model: Option<&NpcModel>,
) -> Option<Handle<ZmoAsset>> {
    if let Some(character_model) = character_model {
        let action = match move_mode {
            MoveMode::Walk => CharacterMotionAction::Walk,
            MoveMode::Run => CharacterMotionAction::Run,
            MoveMode::Drive => todo!("Character drive animation"),
        };

        if character_model.action_motions[action].is_strong() {
            Some(character_model.action_motions[action].clone())
        } else {
            None
        }
    } else if let Some(npc_model) = npc_model {
        let action = match move_mode {
            MoveMode::Walk => NpcMotionAction::Move,
            MoveMode::Run => NpcMotionAction::Run,
            MoveMode::Drive => todo!("NPC drive animation"),
        };

        if npc_model.action_motions[action].is_strong() {
            Some(npc_model.action_motions[action].clone())
        } else {
            None
        }
    } else {
        None
    }
}

fn get_sitting_animation(
    character_model: Option<&CharacterModel>,
    _npc_model: Option<&NpcModel>,
) -> Option<Handle<ZmoAsset>> {
    if let Some(character_model) = character_model {
        if character_model.action_motions[CharacterMotionAction::Sitting].is_strong() {
            Some(character_model.action_motions[CharacterMotionAction::Sitting].clone())
        } else {
            None
        }
    } else {
        None
    }
}

fn get_sit_animation(
    character_model: Option<&CharacterModel>,
    _npc_model: Option<&NpcModel>,
) -> Option<Handle<ZmoAsset>> {
    if let Some(character_model) = character_model {
        if character_model.action_motions[CharacterMotionAction::Sit].is_strong() {
            Some(character_model.action_motions[CharacterMotionAction::Sit].clone())
        } else {
            None
        }
    } else {
        None
    }
}

fn get_standing_animation(
    character_model: Option<&CharacterModel>,
    _npc_model: Option<&NpcModel>,
) -> Option<Handle<ZmoAsset>> {
    if let Some(character_model) = character_model {
        if character_model.action_motions[CharacterMotionAction::Standup].is_strong() {
            Some(character_model.action_motions[CharacterMotionAction::Standup].clone())
        } else {
            None
        }
    } else {
        None
    }
}

fn get_stop_animation(
    character_model: Option<&CharacterModel>,
    npc_model: Option<&NpcModel>,
) -> Option<Handle<ZmoAsset>> {
    if let Some(character_model) = character_model {
        if character_model.action_motions[CharacterMotionAction::Stop1].is_strong() {
            Some(character_model.action_motions[CharacterMotionAction::Stop1].clone())
        } else {
            None
        }
    } else if let Some(npc_model) = npc_model {
        if npc_model.action_motions[NpcMotionAction::Stop].is_strong() {
            Some(npc_model.action_motions[NpcMotionAction::Stop].clone())
        } else {
            None
        }
    } else {
        None
    }
}

fn get_pickup_animation(
    character_model: Option<&CharacterModel>,
    _npc_model: Option<&NpcModel>,
) -> Option<Handle<ZmoAsset>> {
    if let Some(character_model) = character_model {
        if character_model.action_motions[CharacterMotionAction::Pickitem].is_strong() {
            Some(character_model.action_motions[CharacterMotionAction::Pickitem].clone())
        } else {
            None
        }
    } else {
        None
    }
}

fn update_active_motion(
    entity_commands: &mut EntityCommands,
    active_motion: &mut Option<Mut<ActiveMotion>>,
    motion: Handle<ZmoAsset>,
    animation_speed: f32,
    repeat: bool,
) {
    if let Some(active_motion) = active_motion.as_mut() {
        if active_motion.motion == motion {
            // Already playing this animation
            active_motion.animation_speed = animation_speed;
            return;
        }
    }

    entity_commands.insert(if repeat {
        ActiveMotion::new_repeating(motion).with_animation_speed(animation_speed)
    } else {
        ActiveMotion::new_once(motion).with_animation_speed(animation_speed)
    });
}

fn get_attack_animation_speed(ability_values: &AbilityValues) -> f32 {
    i32::max(ability_values.get_attack_speed(), 30) as f32 / 100.0
}

fn get_move_animation_speed(move_speed: &MoveSpeed) -> f32 {
    (move_speed.speed + 180.0) / 600.0
}

pub fn command_system(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        Option<&PlayerCharacter>,
        &AbilityValues,
        Option<&mut ActiveMotion>,
        Option<&CharacterModel>,
        Option<&NpcModel>,
        Option<&Equipment>,
        &Position,
        &MoveMode,
        &MoveSpeed,
        &mut Command,
        &mut NextCommand,
        &mut Transform,
    )>,
    query_move_target: Query<(&Position, &ClientEntity)>,
    query_attack_target: Query<(&Position, &HealthPoints)>,
    query_npc: Query<&Npc>,
    query_personal_store: Query<&PersonalStore>,
    asset_server: Res<AssetServer>,
    game_connection: Option<Res<GameConnection>>,
    game_data: Res<GameData>,
    mut conversation_dialog_events: EventWriter<ConversationDialogEvent>,
    mut client_entity_events: EventWriter<ClientEntityEvent>,
    mut personal_store_events: EventWriter<PersonalStoreEvent>,
) {
    let mut rng = rand::thread_rng();

    for (
        entity,
        player_character,
        ability_values,
        mut active_motion,
        character_model,
        npc_model,
        equipment,
        position,
        move_mode,
        move_speed,
        mut command,
        mut next_command,
        mut transform,
    ) in query.iter_mut()
    {
        let requires_animation_complete = if command.is_emote() {
            // Emote has an animation, but can be interrupted by any other command
            !next_command.is_some()
        } else {
            command.requires_animation_complete()
        };

        if !next_command.is_die() && requires_animation_complete && active_motion.is_some() {
            // Current command still in animation
            continue;
        }

        // Cannot do any commands when dead
        if command.is_die() {
            if npc_model.is_some() {
                // Despawn NPC once the die animation completes
                commands.entity(entity).despawn_recursive();
                continue;
            }

            continue;
        }

        // If sitting animation has complete, set current command to Sit
        if command.is_sitting() {
            if let Some(motion) = get_sit_animation(character_model, npc_model) {
                update_active_motion(
                    &mut commands.entity(entity),
                    &mut active_motion,
                    motion,
                    1.0,
                    true,
                );
            }

            *command = Command::with_sit();
        }

        let weapon_item_data = equipment.and_then(|equipment| {
            equipment
                .get_equipment_item(EquipmentIndex::Weapon)
                .and_then(|weapon_item| {
                    game_data
                        .items
                        .get_weapon_item(weapon_item.item.item_number)
                })
        });
        let weapon_motion_type = weapon_item_data
            .map(|weapon_item_data| weapon_item_data.motion_type as usize)
            .unwrap_or(0);
        let weapon_motion_gender = character_model
            .map(|character_model| match character_model.gender {
                CharacterGender::Male => 0,
                CharacterGender::Female => 1,
            })
            .unwrap_or(0);

        // Handle skill casting transitions
        if let Command::CastSkill(CommandCastSkill {
            cast_skill_state,
            ready_action,
            action_motion_id,
            cast_repeat_motion_id,
            ..
        }) = command.as_mut()
        {
            if *ready_action
                && matches!(
                    *cast_skill_state,
                    CommandCastSkillState::Casting | CommandCastSkillState::CastingRepeat
                )
            {
                if let Some(action_motion_id) = action_motion_id {
                    let motion_data = if let Some(npc_model) = npc_model {
                        game_data
                            .npcs
                            .get_npc_motion(npc_model.npc_id, *action_motion_id)
                    } else {
                        game_data
                            .character_motion_database
                            .find_first_character_motion(
                                *action_motion_id,
                                weapon_motion_type,
                                weapon_motion_gender,
                            )
                    };

                    if let Some(motion_data) = motion_data {
                        update_active_motion(
                            &mut commands.entity(entity),
                            &mut active_motion,
                            asset_server.load(&motion_data.path),
                            1.0,
                            false,
                        );
                    }
                }

                *cast_skill_state = CommandCastSkillState::Action;
                continue;
            } else if !*ready_action && matches!(*cast_skill_state, CommandCastSkillState::Casting)
            {
                if let Some(cast_repeat_motion_id) = cast_repeat_motion_id {
                    let motion_data = if let Some(npc_model) = npc_model {
                        game_data
                            .npcs
                            .get_npc_motion(npc_model.npc_id, *cast_repeat_motion_id)
                    } else {
                        game_data
                            .character_motion_database
                            .find_first_character_motion(
                                *cast_repeat_motion_id,
                                weapon_motion_type,
                                weapon_motion_gender,
                            )
                    };

                    if let Some(motion_data) = motion_data {
                        update_active_motion(
                            &mut commands.entity(entity),
                            &mut active_motion,
                            asset_server.load(&motion_data.path),
                            1.0,
                            true,
                        );
                    }
                }

                *cast_skill_state = CommandCastSkillState::CastingRepeat;
                continue;
            } else if !*ready_action
                && matches!(*cast_skill_state, CommandCastSkillState::CastingRepeat)
            {
                // Repeat CastingRepeat motion until ready_action is true
                continue;
            } else if matches!(cast_skill_state, CommandCastSkillState::Action)
                && next_command.is_none()
            {
                *next_command = NextCommand::with_stop();
            }
        }

        if next_command.is_none() {
            if !command.is_stop() {
                // If we have completed current command, and there is no next command, then clear current.
                // This does not apply for some commands which must be manually completed, such as Sit
                // where you need to stand after.
                if !command.is_manual_complete() {
                    *next_command = NextCommand::with_stop();
                } else {
                    continue;
                }
            } else {
                // Nothing to do, ensure we are using correct idle animation
                if let Some(motion) = get_stop_animation(character_model, npc_model) {
                    update_active_motion(
                        &mut commands.entity(entity),
                        &mut active_motion,
                        motion,
                        1.0,
                        true,
                    );
                }
                continue;
            }
        }

        if command.is_sit() && !next_command.is_die() {
            // If current command is sit, we must stand before performing NextCommand
            if let Some(motion) = get_standing_animation(character_model, npc_model) {
                update_active_motion(
                    &mut commands.entity(entity),
                    &mut active_motion,
                    motion,
                    1.0,
                    false,
                );
            }

            *command = Command::with_standing();
            continue;
        }

        match (*next_command).as_mut().unwrap() {
            Command::Stop => {
                commands
                    .entity(entity)
                    .remove::<Destination>()
                    .remove::<Target>();

                if let Some(motion) = get_stop_animation(character_model, npc_model) {
                    update_active_motion(
                        &mut commands.entity(entity),
                        &mut active_motion,
                        motion,
                        1.0,
                        true,
                    );
                }

                *command = Command::with_stop();
                *next_command = NextCommand::default();
            }
            Command::Move(CommandMove {
                destination,
                target,
                move_mode: command_move_mode,
            }) => {
                let mut entity_commands = commands.entity(entity);
                let mut pickup_item_entity = None;
                let mut talk_to_npc_entity = None;
                let mut move_to_character_entity = None;

                if let Some(target_entity) = target {
                    if let Ok((target_position, target_client_entity)) =
                        query_move_target.get(*target_entity)
                    {
                        let required_distance = match target_client_entity.entity_type {
                            ClientEntityType::Character => {
                                move_to_character_entity =
                                    Some((*target_entity, target_position.position));
                                Some(CHARACTER_MOVE_TO_DISTANCE)
                            }
                            ClientEntityType::Npc => {
                                talk_to_npc_entity =
                                    Some((*target_entity, target_position.position));
                                Some(NPC_MOVE_TO_DISTANCE)
                            }
                            ClientEntityType::ItemDrop => {
                                pickup_item_entity = Some((
                                    *target_entity,
                                    target_client_entity.id,
                                    target_position.position,
                                ));
                                Some(ITEM_DROP_MOVE_TO_DISTANCE)
                            }
                            _ => None,
                        };

                        if let Some(required_distance) = required_distance {
                            let distance = position.position.xy().distance(target_position.xy());
                            if distance < required_distance {
                                // We are already within required distance, so no need to move further
                                *destination = position.position;
                            } else {
                                let offset = (target_position.xy() - position.xy()).normalize()
                                    * required_distance;
                                destination.x = target_position.x - offset.x;
                                destination.y = target_position.y - offset.y;
                                destination.z = target_position.z;
                            }
                        } else {
                            *destination = target_position.position;
                        }
                    } else {
                        *target = None;
                        entity_commands.remove::<Target>();
                    }
                }

                match command_move_mode {
                    Some(MoveMode::Walk) => {
                        if !matches!(move_mode, MoveMode::Walk) {
                            entity_commands
                                .insert(MoveMode::Walk)
                                .insert(MoveSpeed::new(ability_values.get_walk_speed()));
                        }
                    }
                    Some(MoveMode::Run) => {
                        if !matches!(move_mode, MoveMode::Run) {
                            entity_commands
                                .insert(MoveMode::Run)
                                .insert(MoveSpeed::new(ability_values.get_run_speed()));
                        }
                    }
                    Some(MoveMode::Drive) => {
                        if !matches!(move_mode, MoveMode::Drive) {
                            entity_commands
                                .insert(MoveMode::Drive)
                                .insert(MoveSpeed::new(ability_values.get_drive_speed()));
                        }
                    }
                    None => {}
                }

                let distance = position.xy().distance(destination.xy());
                if distance < 0.1 {
                    // Reached destination, stop moving
                    *next_command = NextCommand::with_stop();

                    if player_character.is_some() {
                        // If the player has moved to an item, pick it up
                        if let Some((
                            pickup_item_entity,
                            pickup_item_entity_id,
                            pickup_item_position,
                        )) = pickup_item_entity
                        {
                            // Update rotation to face item
                            let dx = pickup_item_position.x - position.x;
                            let dy = pickup_item_position.y - position.y;
                            transform.rotation = Quat::from_axis_angle(
                                Vec3::Y,
                                dy.atan2(dx) + std::f32::consts::PI / 2.0,
                            );

                            // Ask the server to pick up the item
                            if let Some(game_connection) = game_connection.as_ref() {
                                game_connection
                                    .client_message_tx
                                    .send(ClientMessage::PickupItemDrop(pickup_item_entity_id))
                                    .ok();
                                *next_command = NextCommand::with_pickup_item(pickup_item_entity);
                            }
                        }

                        // If the player has moved to an NPC, open dialog
                        if let Some((talk_to_npc_entity, talk_to_npc_position)) = talk_to_npc_entity
                        {
                            // Update rotation to face NPC
                            let dx = talk_to_npc_position.x - position.x;
                            let dy = talk_to_npc_position.y - position.y;
                            transform.rotation = Quat::from_axis_angle(
                                Vec3::Y,
                                dy.atan2(dx) + std::f32::consts::PI / 2.0,
                            );

                            // Open dialog with npc
                            if let Ok(npc) = query_npc.get(talk_to_npc_entity) {
                                if npc.quest_index != 0 {
                                    if let Some(conversation_data) =
                                        game_data.npcs.find_conversation(npc.quest_index as usize)
                                    {
                                        conversation_dialog_events.send(
                                            ConversationDialogEvent::OpenNpcDialog(
                                                talk_to_npc_entity,
                                                VfsPathBuf::new(&conversation_data.filename),
                                            ),
                                        );
                                    }
                                }
                            }
                        }

                        if let Some((move_to_character_entity, move_to_character_position)) =
                            move_to_character_entity
                        {
                            // Update rotation to face character
                            let dx = move_to_character_position.x - position.x;
                            let dy = move_to_character_position.y - position.y;
                            transform.rotation = Quat::from_axis_angle(
                                Vec3::Y,
                                dy.atan2(dx) + std::f32::consts::PI / 2.0,
                            );

                            // If character is running a personal store, open it
                            if query_personal_store.contains(move_to_character_entity) {
                                personal_store_events.send(PersonalStoreEvent::OpenEntityStore(
                                    move_to_character_entity,
                                ));
                            }
                        }
                    }
                } else {
                    // Move towards destination
                    if let Some(motion) = get_move_animation(move_mode, character_model, npc_model)
                    {
                        update_active_motion(
                            &mut entity_commands,
                            &mut active_motion,
                            motion,
                            get_move_animation_speed(move_speed),
                            true,
                        );
                    }
                    *command = Command::with_move(*destination, *target, *command_move_mode);
                    entity_commands.insert(Destination::new(*destination));

                    if let Some(target_entity) = *target {
                        entity_commands.insert(Target::new(target_entity));
                    }
                }
            }
            &mut Command::Attack(CommandAttack {
                target: target_entity,
            }) => {
                let query_result = query_attack_target.get(target_entity);
                if query_result.is_err() {
                    // Invalid target, stop attacking
                    *next_command = NextCommand::with_stop();
                    continue;
                }
                let (target_position, target_health_points) = query_result.unwrap();

                if target_health_points.hp <= 0 {
                    // Target is dead, stop attacking
                    *next_command = NextCommand::with_stop();
                    continue;
                }

                let mut entity_commands = commands.entity(entity);
                let distance = position.position.xy().distance(target_position.xy());

                let attack_range = ability_values.get_attack_range() as f32;
                if distance < attack_range {
                    // Target in range, start attack
                    if let Some(motion) = get_attack_animation(&mut rng, character_model, npc_model)
                    {
                        // Update rotation to ensure facing enemy
                        let dx = target_position.x - position.x;
                        let dy = target_position.y - position.y;
                        transform.rotation = Quat::from_axis_angle(
                            Vec3::Y,
                            dy.atan2(dx) + std::f32::consts::PI / 2.0,
                        );

                        update_active_motion(
                            &mut entity_commands,
                            &mut active_motion,
                            motion,
                            get_attack_animation_speed(ability_values),
                            false,
                        );
                        *command = Command::with_attack(target_entity);
                        entity_commands.remove::<Destination>();
                        entity_commands.insert(Target::new(target_entity));
                    } else {
                        // No attack animation, stop attack
                        *next_command = NextCommand::default();
                    }
                } else {
                    // Not in range, move towards target
                    let motion = get_move_animation(move_mode, character_model, npc_model);
                    if let Some(motion) = motion {
                        update_active_motion(
                            &mut entity_commands,
                            &mut active_motion,
                            motion,
                            get_move_animation_speed(move_speed),
                            true,
                        );
                        *command = Command::with_move(
                            target_position.position,
                            Some(target_entity),
                            Some(MoveMode::Run),
                        );
                        entity_commands.insert(Destination::new(target_position.position));
                        entity_commands.insert(Target::new(target_entity));
                    } else {
                        // No move animation, stop attack
                        *next_command = NextCommand::default();
                    }
                }
            }
            Command::Die => {
                let motion = get_die_animation(character_model, npc_model);
                if let Some(motion) = motion {
                    update_active_motion(
                        &mut commands.entity(entity),
                        &mut active_motion,
                        motion,
                        1.0,
                        false,
                    );
                }

                client_entity_events.send(ClientEntityEvent::Die(entity));

                *command = Command::with_die();
                *next_command = NextCommand::default();
                commands
                    .entity(entity)
                    .remove::<Destination>()
                    .remove::<Target>();
            }
            &mut Command::PickupItem(item_entity) => {
                if let Ok((target_position, _)) = query_move_target.get(item_entity) {
                    // Update rotation to face pickup item
                    let dx = target_position.x - position.x;
                    let dy = target_position.y - position.y;
                    transform.rotation =
                        Quat::from_axis_angle(Vec3::Y, dy.atan2(dx) + std::f32::consts::PI / 2.0);
                }

                if let Some(motion) = get_pickup_animation(character_model, npc_model) {
                    update_active_motion(
                        &mut commands.entity(entity),
                        &mut active_motion,
                        motion,
                        1.0,
                        false,
                    );
                }

                *command = Command::with_pickup_item(item_entity);
                *next_command = NextCommand::default();
            }
            &mut Command::Emote(CommandEmote { motion_id, is_stop }) => {
                let motion_data = if let Some(npc_model) = npc_model {
                    game_data.npcs.get_npc_motion(npc_model.npc_id, motion_id)
                } else {
                    game_data
                        .character_motion_database
                        .find_first_character_motion(
                            motion_id,
                            weapon_motion_type,
                            weapon_motion_gender,
                        )
                };

                if let Some(motion_data) = motion_data {
                    update_active_motion(
                        &mut commands.entity(entity),
                        &mut active_motion,
                        asset_server.load(&motion_data.path),
                        1.0,
                        false,
                    );
                }

                *command = Command::with_emote(motion_id, is_stop);
                *next_command = NextCommand::default();
                commands
                    .entity(entity)
                    .remove::<Destination>()
                    .remove::<Target>();
            }
            Command::Sit(CommandSit::Sitting) => {
                if let Some(motion) = get_sitting_animation(character_model, npc_model) {
                    update_active_motion(
                        &mut commands.entity(entity),
                        &mut active_motion,
                        motion,
                        1.0,
                        false,
                    );
                }

                *command = Command::with_sitting();
                *next_command = NextCommand::default();
                commands
                    .entity(entity)
                    .remove::<Destination>()
                    .remove::<Target>();
            }
            Command::Sit(CommandSit::Standing) => {
                // The transition from Sit to Standing happens above
                *next_command = NextCommand::default();
            }
            Command::Sit(CommandSit::Sit) => {
                // The transition from Sitting to Sit happens above
                *next_command = NextCommand::default();
            }
            Command::PersonalStore => {
                *command = Command::with_personal_store();
                *next_command = NextCommand::default();
                commands
                    .entity(entity)
                    .remove::<Destination>()
                    .remove::<Target>();
            }
            &mut Command::CastSkill(CommandCastSkill {
                skill_id,
                skill_target,
                cast_motion_id,
                action_motion_id,
                ..
            }) => {
                if let Some(skill_data) = game_data.skills.get_skill(skill_id) {
                    let (target_position, target_entity) = match skill_target {
                        Some(CommandCastSkillTarget::Entity(target_entity)) => {
                            let query_result = query_attack_target.get(target_entity);
                            if query_result.is_err() {
                                // Invalid target, stop casting skill
                                *next_command = NextCommand::with_stop();
                                continue;
                            }
                            (Some(query_result.unwrap().0.position), Some(target_entity))
                        }
                        Some(CommandCastSkillTarget::Position(target_position)) => (
                            Some(Vec3::new(target_position.x, target_position.y, 0.0)),
                            None,
                        ),
                        None => (None, None),
                    };

                    let cast_range = if skill_data.cast_range > 0 {
                        skill_data.cast_range as f32
                    } else {
                        ability_values.get_attack_range() as f32
                    };

                    let in_range = target_position
                        .map(|target_position| {
                            position.xy().distance(target_position.xy()) < cast_range as f32
                        })
                        .unwrap_or(true);
                    if in_range {
                        // Update rotation to face target
                        if let Some(target_position) = target_position.as_ref() {
                            let dx = target_position.x - position.x;
                            let dy = target_position.y - position.y;
                            transform.rotation = Quat::from_axis_angle(
                                Vec3::Y,
                                dy.atan2(dx) + std::f32::consts::PI / 2.0,
                            );
                        }

                        let motion_data =
                            cast_motion_id
                                .or(skill_data.casting_motion_id)
                                .and_then(|motion_id| {
                                    if let Some(npc_model) = npc_model {
                                        game_data.npcs.get_npc_motion(npc_model.npc_id, motion_id)
                                    } else {
                                        game_data
                                            .character_motion_database
                                            .find_first_character_motion(
                                                motion_id,
                                                weapon_motion_type,
                                                weapon_motion_gender,
                                            )
                                    }
                                });

                        if let Some(motion_data) = motion_data {
                            update_active_motion(
                                &mut commands.entity(entity),
                                &mut active_motion,
                                asset_server.load(&motion_data.path),
                                skill_data.casting_motion_speed,
                                false,
                            );
                        }

                        // Update next command
                        match skill_data.action_mode {
                            SkillActionMode::Stop => *next_command = NextCommand::default(),
                            SkillActionMode::Attack => {
                                *next_command = target_entity
                                    .map_or_else(NextCommand::default, |target| {
                                        NextCommand::with_attack(target)
                                    });
                            }
                            SkillActionMode::Restore => match *command {
                                Command::Stop | Command::Move(_) | Command::Attack(_) => {
                                    *next_command = NextCommand::new(Some(command.clone()));
                                }
                                Command::Die
                                | Command::Emote(_)
                                | Command::PickupItem(_)
                                | Command::PersonalStore
                                | Command::Sit(_)
                                | Command::CastSkill(_) => *next_command = NextCommand::default(),
                            },
                        }

                        // Set current command to cast skill
                        *command = Command::with_cast_skill(
                            skill_id,
                            skill_target,
                            cast_motion_id.or(skill_data.casting_motion_id),
                            skill_data.casting_repeat_motion_id,
                            action_motion_id.or(skill_data.action_motion_id),
                            CommandCastSkillState::Casting,
                        );

                        // Remove our destination component, as we have reached it!
                        commands.entity(entity).remove::<Destination>();
                    } else {
                        let mut entity_commands = commands.entity(entity);
                        let target_position = target_position.unwrap();

                        // Not in range, move towards target
                        let motion = get_move_animation(move_mode, character_model, npc_model);
                        if let Some(motion) = motion {
                            update_active_motion(
                                &mut entity_commands,
                                &mut active_motion,
                                motion,
                                get_move_animation_speed(move_speed),
                                false,
                            );
                            *command = Command::with_move(
                                target_position,
                                target_entity,
                                Some(MoveMode::Run),
                            );
                            entity_commands.insert(Destination::new(target_position));

                            if let Some(target_entity) = target_entity {
                                entity_commands.insert(Target::new(target_entity));
                            }
                        } else {
                            // No move animation, stop attack
                            *next_command = NextCommand::default();
                        }
                    }
                } else {
                    *next_command = NextCommand::default();
                }
            }
        }
    }
}
