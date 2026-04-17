use super::CosItem;
use crate::config::AppConfig;
use crate::error::{Error, Result};
use crate::storage::Storage;
use crate::utils::resolve_path;
use chrono::{DateTime, Utc};
use glob::glob;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::info;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocalStorageItem {
    pub key: String,
    pub last_modified: DateTime<Utc>,
    pub size: u64,
}

impl From<LocalStorageItem> for CosItem {
    fn from(item: LocalStorageItem) -> Self {
        CosItem {
            key: item.key,
            last_modified: item.last_modified,
            size: item.size,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LocalStorage {
    pub base_path: PathBuf,
}

#[async_trait::async_trait]
impl Storage for LocalStorage {
    async fn upload(&self, _file_path: &Path, _cos_path: &str) -> Result<()> {
        Ok(())
    }

    async fn list(&self, key: &str) -> Result<Vec<CosItem>> {
        let pattern = self.base_path.join(key).to_string_lossy().to_string();
        let mut items = Vec::new();

        let files = glob(&pattern).map_err(|e| Error::StorageList(e.to_string()))?;

        for path in files.flatten() {
            let metadata = fs::metadata(&path)
                .await
                .map_err(|e| Error::StorageList(e.to_string()))?;
            if metadata.is_file() {
                let last_modified: DateTime<Utc> = metadata
                    .modified()
                    .map_err(|e| {
                        Error::StorageList(format!("Failed to get modification time: {}", e))
                    })?
                    .into();

                let file_name = path
                    .file_name()
                    .ok_or_else(|| {
                        Error::StorageList(format!("Invalid file path: {}", path.display()))
                    })?
                    .to_string_lossy();

                items.push(LocalStorageItem {
                    key: file_name.to_string(),
                    last_modified,
                    size: metadata.len(),
                });
            }
        }

        items.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));

        Ok(items.into_iter().map(CosItem::from).collect())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let target_path = self.base_path.join(key);

        if !target_path.exists() {
            return Err(Error::FileNotFound(target_path));
        }

        fs::remove_file(&target_path)
            .await
            .map_err(|e| Error::StorageDelete {
                key: key.to_string(),
                message: e.to_string(),
            })?;

        info!("Successfully deleted: {}", target_path.display());
        Ok(())
    }
}

impl LocalStorage {
    pub async fn new(base_path: &str) -> Self {
        let path = resolve_path(base_path);
        let backup_path = match path {
            Ok(p) => p,
            Err(_) => AppConfig::default().backup_dir,
        };
        let _ = fs::create_dir_all(&backup_path).await.map_err(|_| {});
        LocalStorage {
            base_path: backup_path,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_local_storage_upload() {
        let temp_dir = tempdir().unwrap();
        let storage = LocalStorage::new(temp_dir.path().to_str().unwrap()).await;

        let test_file = temp_dir.path().join("test.txt");
        let mut file = File::create(&test_file).await.unwrap();
        file.write_all(b"test content").await.unwrap();

        let result = storage.upload(&test_file, "uploads/").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_local_storage_list() {
        let temp_dir = tempdir().unwrap();
        let storage = LocalStorage::new(temp_dir.path().to_str().unwrap()).await;

        let test_file = temp_dir.path().join("test.txt");
        let mut file = File::create(&test_file).await.unwrap();
        file.write_all(b"test content").await.unwrap();

        let items = storage.list("*.txt").await.unwrap();
        assert!(!items.is_empty());
        assert_eq!(items[0].key, "test.txt");
    }

    #[tokio::test]
    async fn test_local_storage_delete() {
        let temp_dir = tempdir().unwrap();
        let storage = LocalStorage::new(temp_dir.path().to_str().unwrap()).await;

        let test_file = temp_dir.path().join("test.txt");
        let mut file = File::create(&test_file).await.unwrap();
        file.write_all(b"test content").await.unwrap();

        let result = storage.delete("test.txt").await;
        assert!(result.is_ok());
        assert!(!test_file.exists());
    }
}
