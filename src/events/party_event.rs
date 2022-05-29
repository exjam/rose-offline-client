use bevy::prelude::Entity;

pub enum PartyEvent {
    InvitedCreate(Entity),
    InvitedJoin(Entity),
}
