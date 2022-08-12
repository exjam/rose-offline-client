use bevy::{
    math::{Vec2, Vec3},
    prelude::{Component, Entity},
};
use rose_data::{MotionId, SkillId};
use std::ops::{Deref, DerefMut};

use rose_game_common::components::MoveMode;

#[derive(Clone, Debug, PartialEq)]
pub struct CommandMove {
    pub destination: Vec3,
    pub target: Option<Entity>,
    pub move_mode: Option<MoveMode>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandAttack {
    pub target: Entity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandEmote {
    pub motion_id: MotionId,
    pub is_stop: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandSit {
    Sitting,
    Sit,
    Standing,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum CommandCastSkillTarget {
    Entity(Entity),
    Position(Vec2),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CommandCastSkillState {
    Starting,
    Casting,
    CastingRepeat,
    Action,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CommandCastSkill {
    pub skill_id: SkillId,
    pub skill_target: Option<CommandCastSkillTarget>,
    pub action_motion_id: Option<MotionId>,
    pub cast_motion_id: Option<MotionId>,
    pub cast_repeat_motion_id: Option<MotionId>,
    pub cast_skill_state: CommandCastSkillState,
    pub ready_action: bool,
}

#[derive(Component, Clone, Debug, PartialEq)]
pub enum Command {
    Stop,
    Move(CommandMove),
    Attack(CommandAttack),
    Die,
    PersonalStore,
    PickupItem(Entity),
    Emote(CommandEmote),
    Sit(CommandSit),
    CastSkill(CommandCastSkill),
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

    pub fn with_cast_skill(
        skill_id: SkillId,
        skill_target: Option<CommandCastSkillTarget>,
        cast_motion_id: Option<MotionId>,
        cast_repeat_motion_id: Option<MotionId>,
        action_motion_id: Option<MotionId>,
        cast_skill_state: CommandCastSkillState,
    ) -> Self {
        Self::CastSkill(CommandCastSkill {
            skill_id,
            skill_target,
            cast_motion_id,
            cast_repeat_motion_id,
            action_motion_id,
            cast_skill_state,
            ready_action: false,
        })
    }

    pub fn with_emote(motion_id: MotionId, is_stop: bool) -> Self {
        Self::Emote(CommandEmote { motion_id, is_stop })
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

    pub fn with_personal_store() -> Self {
        Self::PersonalStore
    }

    pub fn with_pickup_item(target: Entity) -> Self {
        Self::PickupItem(target)
    }

    pub fn with_sitting() -> Self {
        Self::Sit(CommandSit::Sitting)
    }

    pub fn with_sit() -> Self {
        Self::Sit(CommandSit::Sit)
    }

    pub fn with_standing() -> Self {
        Self::Sit(CommandSit::Standing)
    }

    pub fn is_die(&self) -> bool {
        matches!(self, Command::Die)
    }

    pub fn is_emote(&self) -> bool {
        matches!(self, Command::Emote(_))
    }

    pub fn is_stop(&self) -> bool {
        matches!(self, Command::Stop)
    }

    pub fn is_sitting(&self) -> bool {
        matches!(self, Command::Sit(CommandSit::Sitting))
    }

    pub fn is_sit(&self) -> bool {
        matches!(self, Command::Sit(CommandSit::Sit))
    }

    pub fn is_manual_complete(&self) -> bool {
        matches!(
            self,
            Command::Sit(_) | Command::CastSkill(_) | Command::PersonalStore
        )
    }

    pub fn requires_animation_complete(&self) -> bool {
        match self {
            Command::Stop => false,
            Command::Move(_) => false,
            Command::Attack(_) => true,
            Command::Die => true,
            Command::PickupItem(_) => true,
            Command::Emote(_) => true,
            Command::Sit(CommandSit::Sitting) => true,
            Command::Sit(CommandSit::Sit) => false,
            Command::Sit(CommandSit::Standing) => true,
            Command::CastSkill(cast_skill) => !matches!(
                cast_skill.cast_skill_state,
                CommandCastSkillState::CastingRepeat
            ),
            Command::PersonalStore => false,
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

    pub fn with_attack(target: Entity) -> Self {
        Self(Some(Command::Attack(CommandAttack { target })))
    }

    pub fn with_cast_skill(
        skill_id: SkillId,
        skill_target: Option<CommandCastSkillTarget>,
        cast_motion_id: Option<MotionId>,
        cast_repeat_motion_id: Option<MotionId>,
        action_motion_id: Option<MotionId>,
    ) -> Self {
        Self(Some(Command::CastSkill(CommandCastSkill {
            skill_id,
            skill_target,
            cast_motion_id,
            cast_repeat_motion_id,
            action_motion_id,
            cast_skill_state: CommandCastSkillState::Starting,
            ready_action: false,
        })))
    }

    pub fn with_die() -> Self {
        Self(Some(Command::Die))
    }

    pub fn with_emote(motion_id: MotionId, is_stop: bool) -> Self {
        Self(Some(Command::Emote(CommandEmote { motion_id, is_stop })))
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

    pub fn with_personal_store() -> Self {
        Self(Some(Command::PersonalStore))
    }

    pub fn with_pickup_item(target: Entity) -> Self {
        Self(Some(Command::PickupItem(target)))
    }

    pub fn with_sitting() -> Self {
        Self(Some(Command::Sit(CommandSit::Sitting)))
    }

    pub fn with_standing() -> Self {
        Self(Some(Command::Sit(CommandSit::Standing)))
    }

    pub fn with_stop() -> Self {
        Self(Some(Command::Stop))
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
