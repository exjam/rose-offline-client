use bevy::prelude::{Entity, Event};

use rose_file_readers::VfsPathBuf;

#[derive(Event)]
pub enum ConversationDialogEvent {
    OpenNpcDialog(Entity, VfsPathBuf),
    OpenEventDialog(VfsPathBuf),
}
