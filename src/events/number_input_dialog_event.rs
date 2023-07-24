use bevy::prelude::{Commands, Event};

#[derive(Event)]
pub enum NumberInputDialogEvent {
    Show {
        max_value: Option<usize>,
        modal: bool,
        ok: Option<Box<dyn FnOnce(&mut Commands, usize) + Send + Sync>>,
        cancel: Option<Box<dyn FnOnce(&mut Commands) + Send + Sync>>,
    },
}
