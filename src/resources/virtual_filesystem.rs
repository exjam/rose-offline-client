use std::sync::Arc;

use rose_file_readers::VirtualFilesystem;

pub struct VfsResource {
    pub vfs: Arc<VirtualFilesystem>,
}
