use crate::error::ApplicationResult;
use crate::ports::ModRepository;
use cubed_domain::entities::ModEntry;
use std::sync::Arc;
use uuid::Uuid;

pub struct ListMods {
    mods: Arc<dyn ModRepository>,
}

impl ListMods {
    pub fn new(mods: Arc<dyn ModRepository>) -> Self {
        Self { mods }
    }

    pub async fn execute(&self, server_id: Uuid) -> ApplicationResult<Vec<ModEntry>> {
        let mut list = self.mods.find_by_server(server_id).await?;
        list.sort_by(|a, b| a.file_name().cmp(b.file_name()));
        Ok(list)
    }
}
