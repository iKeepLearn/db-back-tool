pub mod mysql;
pub mod postgresql;

use std::path::{Path, PathBuf};

#[async_trait::async_trait]
pub trait Database {
    async fn backup(&self, database_name: &str, backup_dir: &Path) -> anyhow::Result<PathBuf>;
}
