use bevy::prelude::{Commands, Event};

#[derive(Event)]
pub enum MessageBoxEvent {
    Show {
        message: String,
        modal: bool,
        ok: Option<Box<dyn FnOnce(&mut Commands) + Send + Sync>>,
        cancel: Option<Box<dyn FnOnce(&mut Commands) + Send + Sync>>,
    },
}
