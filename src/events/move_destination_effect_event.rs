use bevy::prelude::Vec3;

pub enum MoveDestinationEffectEvent {
    Show { position: Vec3 },
    Hide,
}
