use bevy::prelude::Commands;

pub enum MessageBoxEvent {
    Show {
        message: String,
        modal: bool,
        ok: Option<Box<dyn FnOnce(&mut Commands) + Send + Sync>>,
        cancel: Option<Box<dyn FnOnce(&mut Commands) + Send + Sync>>,
    },
}
