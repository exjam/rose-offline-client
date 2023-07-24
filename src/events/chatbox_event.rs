use bevy::prelude::Event;

#[derive(Event)]
pub enum ChatboxEvent {
    Say(String, String),
    Shout(String, String),
    Whisper(String, String),
    Announce(Option<String>, String),
    System(String),
    Quest(String),
}
