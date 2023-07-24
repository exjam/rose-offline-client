use bevy::prelude::Event;

use rose_data::ZoneId;

#[derive(Event)]
pub enum GameConnectionEvent {
    Connected(ZoneId),
}
