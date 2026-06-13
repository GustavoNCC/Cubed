use crate::error::ApplicationResult;
use crate::ports::FileSystemManager;
use std::sync::Arc;

pub struct InitFileSystem {
    fs: Arc<dyn FileSystemManager>,
}

impl InitFileSystem {
    pub fn new(fs: Arc<dyn FileSystemManager>) -> Self {
        Self { fs }
    }

    pub async fn execute(&self) -> ApplicationResult<()> {
        self.fs.init_cubed_dirs().await
    }
}
