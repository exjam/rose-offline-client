use bevy::prelude::{Event, Vec3};

#[derive(Event)]
pub enum MoveDestinationEffectEvent {
    Show { position: Vec3 },
    Hide,
}
