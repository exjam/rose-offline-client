use bevy::{
    math::Vec3,
    prelude::{Component, Entity},
};
use std::time::Duration;

use rose_game_common::components::MoveMode;

#[derive(Clone, Debug)]
pub struct CommandMove {
    pub destination: Vec3,
    pub target: Option<Entity>,
    pub move_mode: Option<MoveMode>,
}

#[derive(Clone, Debug)]
pub struct CommandAttack {
    pub target: Entity,
}

#[derive(Clone, Debug)]
pub enum CommandData {
    Stop,
    Move(CommandMove),
    Attack(CommandAttack),
    Die,
}

#[derive(Component, Clone, Debug)]
pub struct Command {
    // Current command that is executing
    pub command: CommandData,

    // How long the current command has been executing
    pub duration: Duration,

    // The duration required to complete this command, if None then the command is immediately interruptible
    pub required_duration: Option<Duration>,
}

impl Command {
    pub fn new(command: CommandData, required_duration: Option<Duration>) -> Self {
        Self {
            command,
            duration: Duration::new(0, 0),
            required_duration,
        }
    }

    pub fn default() -> Self {
        Self::with_stop()
    }

    pub fn with_stop() -> Self {
        Self::new(CommandData::Stop, None)
    }

    pub fn with_die(required_duration: Duration) -> Self {
        Self::new(CommandData::Die, Some(required_duration))
    }

    pub fn with_attack(target: Entity, duration: Duration) -> Self {
        Self::new(
            CommandData::Attack(CommandAttack { target }),
            Some(duration),
        )
    }

    pub fn with_move(
        destination: Vec3,
        target: Option<Entity>,
        move_mode: Option<MoveMode>,
    ) -> Self {
        Self::new(
            CommandData::Move(CommandMove {
                destination,
                target,
                move_mode,
            }),
            None,
        )
    }
}

#[derive(Component, Default)]
pub struct NextCommand {
    pub command: Option<CommandData>,
}

impl NextCommand {
    #[allow(dead_code)]
    pub fn with_stop() -> Self {
        Self {
            command: Some(CommandData::Stop),
        }
    }

    pub fn with_move(
        destination: Vec3,
        target: Option<Entity>,
        move_mode: Option<MoveMode>,
    ) -> Self {
        Self {
            command: Some(CommandData::Move(CommandMove {
                destination,
                target,
                move_mode,
            })),
        }
    }

    pub fn with_attack(target: Entity) -> Self {
        Self {
            command: Some(CommandData::Attack(CommandAttack { target })),
        }
    }
}
