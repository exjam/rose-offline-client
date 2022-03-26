use std::time::Duration;

use bevy::{
    core::Time,
    math::Vec3Swizzles,
    prelude::{Commands, Entity, Query, Res},
};
use rose_game_common::components::{AbilityValues, Destination, MoveMode, MoveSpeed, Target};

use crate::components::{
    CharacterModel, Command, CommandAttack, CommandData, CommandMove, NextCommand, NpcModel,
    Position,
};

pub fn command_system(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &AbilityValues,
        Option<&CharacterModel>,
        Option<&NpcModel>,
        &Position,
        &MoveMode,
        &mut Command,
        &mut NextCommand,
    )>,
    query_position: Query<&Position>,
    time: Res<Time>,
) {
    for (
        entity,
        ability_values,
        character_model,
        npc_model,
        position,
        move_mode,
        mut command,
        mut next_command,
    ) in query.iter_mut()
    {
        command.duration += Duration::from_secs_f64(time.delta_seconds_f64());

        let required_duration = match &mut command.command {
            CommandData::Attack(_) => {
                let attack_speed = i32::max(ability_values.get_attack_speed(), 30) as f32 / 100.0;
                command
                    .required_duration
                    .map(|duration| duration.div_f32(attack_speed))
            }
            _ => command.required_duration,
        };

        // Some commands require the whole animation to complete before we can move to next command
        let command_motion_completed = required_duration.map_or(true, |required_duration| {
            command.duration >= required_duration
        });
        if !command_motion_completed {
            // Current command still in animation
            continue;
        }

        if next_command.command.is_none() {
            // We have completed current command and there is no next command, so clear any current.
            *command = Command::default();

            // Nothing to do when there is no next command
            continue;
        }

        match next_command.command.as_mut().unwrap() {
            CommandData::Stop => {
                commands
                    .entity(entity)
                    .remove::<Destination>()
                    .remove::<Target>();
                *command = Command::with_stop();
                *next_command = NextCommand::default();
            }
            CommandData::Move(CommandMove {
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
                    *command = Command::with_stop();
                    entity_commands.remove::<Target>().remove::<Destination>();
                } else {
                    *command = Command::with_move(*destination, *target, *command_move_mode);
                    entity_commands.insert(Destination::new(*destination));

                    if let Some(target_entity) = *target {
                        entity_commands.insert(Target::new(target_entity));
                    }
                }
            }
            &mut CommandData::Attack(CommandAttack {
                target: target_entity,
            }) => {
                if let Ok(target_position) = query_position.get(target_entity) {
                    let mut entity_commands = commands.entity(entity);
                    let distance = position
                        .position
                        .xy()
                        .distance(target_position.position.xy());

                    // Check if we are in attack range
                    let attack_range = ability_values.get_attack_range() as f32;
                    if distance < attack_range {
                        let (attack_duration, hit_count) =
                            if let Some(character_model) = character_model {
                                // TODO: duration of character_model.action_motions[CharacterMotionAction::Attack];
                                (Duration::from_secs(1), 1)
                            } else if let Some(npc_model) = npc_model {
                                // TODO: NPC attack duration !
                                (Duration::from_secs(1), 1)
                            } else {
                                (Duration::from_secs(1), 1)
                            };

                        // TODO: If the weapon uses ammo, we must consume the ammo
                        let mut cancel_attack = false;
                        /*
                        if let Some(mut equipment) = equipment {
                            if let Some(weapon_data) = equipment
                                .get_equipment_item(EquipmentIndex::WeaponRight)
                                .and_then(|weapon_item| {
                                    game_data.items.get_base_item(weapon_item.item)
                                })
                            {
                                let ammo_index = match weapon_data.class {
                                    ItemClass::Bow | ItemClass::Crossbow => Some(AmmoIndex::Arrow),
                                    ItemClass::Gun | ItemClass::DualGuns => Some(AmmoIndex::Bullet),
                                    ItemClass::Launcher => Some(AmmoIndex::Throw),
                                    _ => None,
                                };

                                if let Some(ammo_index) = ammo_index {
                                    if equipment
                                        .get_ammo_slot_mut(ammo_index)
                                        .try_take_quantity(hit_count as u32)
                                        .is_none()
                                    {
                                        cancel_attack = true;
                                    } else if let Some(game_client) = game_client {
                                        match equipment.get_ammo_item(ammo_index) {
                                            Some(ammo_item) => {
                                                if (ammo_item.quantity & 0x0F) == 0 {
                                                    game_client
                                                        .server_message_tx
                                                        .send(ServerMessage::UpdateInventory(
                                                            vec![(
                                                                ItemSlot::Ammo(ammo_index),
                                                                Some(Item::Stackable(
                                                                    ammo_item.clone(),
                                                                )),
                                                            )],
                                                            None,
                                                        ))
                                                        .ok();
                                                }
                                            }
                                            None => {
                                                server_messages.send_entity_message(
                                                    client_entity,
                                                    ServerMessage::UpdateAmmo(
                                                        client_entity.id,
                                                        ammo_index,
                                                        None,
                                                    ),
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        */

                        if cancel_attack {
                            // Not enough ammo, cancel attack
                            *next_command = NextCommand::default();
                        } else {
                            // In range, set current command to attack
                            *command = Command::with_attack(target_entity, attack_duration);

                            // Remove our destination component, as we have reached it!
                            entity_commands.remove::<Destination>();

                            // Update target
                            entity_commands.insert(Target::new(target_entity));
                        }
                    } else {
                        // Not in range, set current command to move
                        *command = Command::with_move(
                            target_position.position,
                            Some(target_entity),
                            Some(MoveMode::Run),
                        );

                        // Set destination to move towards
                        entity_commands.insert(Destination::new(target_position.position));

                        // Update target
                        entity_commands.insert(Target::new(target_entity));
                    }
                } else {
                    // TODO: Do we send a stop command ?
                    *next_command = NextCommand::default();
                }
            }
        }
    }
}
