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
pub enum CommandData {
    Stop,
    Move(CommandMove),
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

impl Default for Command {
    fn default() -> Self {
        Self {
            command: CommandData::Stop,
            duration: Duration::default(),
            required_duration: None,
        }
    }
}

impl Command {
    pub fn with_stop() -> Self {
        Self {
            command: CommandData::Stop,
            duration: Duration::default(),
            required_duration: None,
        }
    }

    pub fn with_move(
        destination: Vec3,
        target: Option<Entity>,
        move_mode: Option<MoveMode>,
    ) -> Self {
        Self {
            command: CommandData::Move(CommandMove {
                destination,
                target,
                move_mode,
            }),
            duration: Duration::default(),
            required_duration: None,
        }
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
}
