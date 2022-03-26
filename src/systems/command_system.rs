use bevy::{
    core::Time,
    ecs::system::EntityCommands,
    hierarchy::DespawnRecursiveExt,
    math::Vec3Swizzles,
    prelude::{Commands, Entity, Handle, Query, Res},
};
use rand::prelude::SliceRandom;

use rose_data::{CharacterMotionAction, NpcMotionAction};
use rose_game_common::components::{
    AbilityValues, Destination, HealthPoints, MoveMode, MoveSpeed, Target,
};

use crate::{
    components::{
        ActiveMotion, CharacterModel, Command, CommandAttack, CommandMove, NextCommand, NpcModel,
        Position,
    },
    zmo_asset_loader::ZmoAsset,
};

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

fn update_active_motion(
    entity_commands: &mut EntityCommands,
    active_motion: Option<&ActiveMotion>,
    motion: Handle<ZmoAsset>,
    time: &Time,
    repeat: bool,
) {
    if active_motion.map_or(false, |x| x.motion == motion) {
        // Already playing this animation
        return;
    }

    entity_commands.insert(if repeat {
        ActiveMotion::new_repeating(motion, time.seconds_since_startup())
    } else {
        ActiveMotion::new_once(motion, time.seconds_since_startup())
    });
}

pub fn command_system(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &AbilityValues,
        Option<&ActiveMotion>,
        Option<&CharacterModel>,
        Option<&NpcModel>,
        &Position,
        &MoveMode,
        &mut Command,
        &mut NextCommand,
    )>,
    query_position: Query<&Position>,
    query_attack_target: Query<(&Position, &HealthPoints)>,
    time: Res<Time>,
) {
    let mut rng = rand::thread_rng();

    for (
        entity,
        ability_values,
        active_motion,
        character_model,
        npc_model,
        position,
        move_mode,
        mut command,
        mut next_command,
    ) in query.iter_mut()
    {
        if !next_command.is_die()
            && command.requires_animation_complete()
            && !active_motion.map_or(true, |x| x.complete)
        {
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

        if next_command.is_none() {
            if !command.is_stop() {
                // Set current command to stop
                *next_command = NextCommand::with_stop();
            } else {
                // Nothing to do, ensure we are using correct idle animation
                if let Some(motion) = get_stop_animation(character_model, npc_model) {
                    update_active_motion(
                        &mut commands.entity(entity),
                        active_motion,
                        motion,
                        &time,
                        true,
                    );
                }
                continue;
            }
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
                        active_motion,
                        motion,
                        &time,
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

                if let Some(target_entity) = target {
                    if let Ok(target_position) = query_position.get(*target_entity) {
                        let required_distance = Some(250.0);

                        if let Some(required_distance) = required_distance {
                            let offset = (target_position.position.xy() - position.position.xy())
                                .normalize()
                                * required_distance;
                            destination.x = target_position.position.x - offset.x;
                            destination.y = target_position.position.y - offset.y;
                            destination.z = target_position.position.z;
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

                let distance = position.position.xy().distance(destination.xy());
                if distance < 0.1 {
                    // Reached destination, stop moving
                    *next_command = NextCommand::with_stop();
                } else {
                    // Move towards destination
                    if let Some(motion) = get_move_animation(move_mode, character_model, npc_model)
                    {
                        update_active_motion(
                            &mut entity_commands,
                            active_motion,
                            motion,
                            &time,
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
                let distance = position
                    .position
                    .xy()
                    .distance(target_position.position.xy());

                let attack_range = ability_values.get_attack_range() as f32;
                if distance < attack_range {
                    // Target in range, start attack
                    if let Some(motion) = get_attack_animation(&mut rng, character_model, npc_model)
                    {
                        // TODO: Apply attack speed multiplier to motion playback speed
                        update_active_motion(
                            &mut entity_commands,
                            active_motion,
                            motion,
                            &time,
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
                            active_motion,
                            motion,
                            &time,
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
                        active_motion,
                        motion,
                        &time,
                        false,
                    );
                }

                *command = Command::with_die();
                *next_command = NextCommand::default();
            }
        }
    }
}
