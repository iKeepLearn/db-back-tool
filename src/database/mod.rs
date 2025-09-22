pub mod postgresql;

use std::path::PathBuf;

#[async_trait::async_trait]
pub trait Database {
    async fn backup(&self, database_name: &str, backup_dir: &PathBuf) -> anyhow::Result<PathBuf>;
}
