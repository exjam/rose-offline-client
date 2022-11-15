use bevy::prelude::Resource;
use std::sync::Arc;

use rose_file_readers::VirtualFilesystem;

#[derive(Resource)]
pub struct VfsResource {
    pub vfs: Arc<VirtualFilesystem>,
}
