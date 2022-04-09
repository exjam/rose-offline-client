use bevy::{
    math::Vec3,
    prelude::{Component, Entity},
};
use rose_data::MotionId;
use std::ops::{Deref, DerefMut};

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
pub struct CommandEmote {
    pub motion_id: MotionId,
    pub is_stop: bool,
}

#[derive(Component, Clone, Debug)]
pub enum Command {
    Stop,
    Move(CommandMove),
    Attack(CommandAttack),
    Die,
    PickupItem(Entity),
    Emote(CommandEmote),
}

impl Command {
    pub fn with_die() -> Self {
        Self::Die
    }

    pub fn with_stop() -> Self {
        Self::Stop
    }

    pub fn with_attack(target: Entity) -> Self {
        Self::Attack(CommandAttack { target })
    }

    pub fn with_emote(motion_id: MotionId, is_stop: bool) -> Self {
        Self::Emote(CommandEmote { motion_id, is_stop })
    }

    pub fn with_pickup_item(target: Entity) -> Self {
        Self::PickupItem(target)
    }

    pub fn with_move(
        destination: Vec3,
        target: Option<Entity>,
        move_mode: Option<MoveMode>,
    ) -> Self {
        Self::Move(CommandMove {
            destination,
            target,
            move_mode,
        })
    }

    pub fn is_die(&self) -> bool {
        matches!(self, Command::Die)
    }

    pub fn is_stop(&self) -> bool {
        matches!(self, Command::Stop)
    }

    pub fn is_sit(&self) -> bool {
        false
    }

    pub fn requires_animation_complete(&self) -> bool {
        match self {
            Command::Stop => false,
            Command::Move(_) => false,
            Command::Attack(_) => true,
            Command::Die => true,
            Command::PickupItem(_) => true,
            Command::Emote(_) => true,
        }
    }
}

#[derive(Component)]
pub struct NextCommand(Option<Command>);

impl NextCommand {
    pub fn new(command: Option<Command>) -> Self {
        Self(command)
    }

    pub fn default() -> Self {
        Self::new(None)
    }

    pub fn is_die(&self) -> bool {
        matches!(self.0, Some(Command::Die))
    }

    pub fn with_die() -> Self {
        Self(Some(Command::Die))
    }

    pub fn with_stop() -> Self {
        Self(Some(Command::Stop))
    }

    pub fn with_pickup_item(target: Entity) -> Self {
        Self(Some(Command::PickupItem(target)))
    }

    pub fn with_move(
        destination: Vec3,
        target: Option<Entity>,
        move_mode: Option<MoveMode>,
    ) -> Self {
        Self(Some(Command::Move(CommandMove {
            destination,
            target,
            move_mode,
        })))
    }

    pub fn with_emote(motion_id: MotionId, is_stop: bool) -> Self {
        Self(Some(Command::Emote(CommandEmote { motion_id, is_stop })))
    }

    pub fn with_attack(target: Entity) -> Self {
        Self(Some(Command::Attack(CommandAttack { target })))
    }
}

impl Deref for NextCommand {
    type Target = Option<Command>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for NextCommand {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
