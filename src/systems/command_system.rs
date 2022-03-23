use std::time::Duration;

use bevy::{
    core::Time,
    math::Vec3Swizzles,
    prelude::{Commands, Entity, Query, Res},
};
use rose_game_common::components::{AbilityValues, Destination, MoveMode, MoveSpeed, Target};

use crate::components::{Command, CommandData, CommandMove, NextCommand, Position};

#[allow(clippy::type_complexity)]
pub fn command_system(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &AbilityValues,
        &Position,
        &MoveMode,
        &mut Command,
        &mut NextCommand,
    )>,
    query_position: Query<&Position>,
    time: Res<Time>,
) {
    for (entity, ability_values, position, move_mode, mut command, mut next_command) in
        query.iter_mut()
    {
        command.duration += Duration::from_secs_f64(time.delta_seconds_f64());

        // Some commands require the whole animation to complete before we can move to next command
        let command_motion_completed =
            command.required_duration.map_or(true, |required_duration| {
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
        }
    }
}
