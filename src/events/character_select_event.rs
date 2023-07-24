use bevy::prelude::Event;

#[derive(Event)]
pub enum CharacterSelectEvent {
    SelectCharacter(usize),
    PlaySelected,
    DeleteSelected,
    Disconnect,
}
