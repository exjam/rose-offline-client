use bevy::prelude::{Entity, Event};

#[derive(Event)]
pub enum PartyEvent {
    InvitedCreate(Entity),
    InvitedJoin(Entity),
}
