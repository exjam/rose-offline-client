use bevy::prelude::Entity;

use rose_file_readers::VfsPathBuf;

pub enum ConversationDialogEvent {
    OpenNpcDialog(Entity, VfsPathBuf),
    OpenEventDialog(VfsPathBuf),
}
