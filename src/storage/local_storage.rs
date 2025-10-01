use super::CosItem;
use crate::{config::AppConfig, storage::Storage, utils::resolve_path};
use anyhow::Result;
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

pub struct LocalStorage {
    pub base_path: PathBuf,
}

#[async_trait::async_trait]
impl Storage for LocalStorage {
    async fn upload(&self, _file_path: &Path, _cos_path: &str) -> Result<(), String> {
        // // 获取文件名
        // let file_name = file_path
        //     .file_name()
        //     .ok_or_else(|| format!("Invalid file path: {}", file_path.display()))?
        //     .to_string_lossy();

        // // 构建目标路径
        // let target_path = self.base_path.join(&file_name.as_ref());
        // fs::copy(file_path, target_path.as_path())
        //     .await
        //     .map_err(|e| format!("Failed to copy file: {}", e))?;
        // info!(
        //     "Successfully uploaded: {} to {}",
        //     file_name,
        //     target_path.display()
        // );
        Ok(())
    }

    async fn list(&self, key: &str) -> Result<Vec<CosItem>, String> {
        let pattern = self.base_path.join(key).to_string_lossy().to_string();
        let mut items = Vec::new();

        let files = glob(&pattern).map_err(|e| e.to_string())?;

        for entry in files {
            if let Ok(path) = entry {
                let metadata = fs::metadata(&path).await.map_err(|e| e.to_string())?;
                if metadata.is_file() {
                    let last_modified: DateTime<Utc> = metadata
                        .modified()
                        .map_err(|e| format!("Failed to get modification time: {}", e))?
                        .into();

                    let file_name = path
                        .file_name()
                        .ok_or_else(|| format!("Invalid file path: {}", path.display()))?
                        .to_string_lossy();

                    items.push(LocalStorageItem {
                        key: file_name.to_string(),
                        last_modified,
                        size: metadata.len(),
                    });
                }
            }
        }

        // 按最后修改时间排序（最新的在前）
        items.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));

        Ok(items.into_iter().map(CosItem::from).collect())
    }

    async fn delete(&self, key: &str) -> Result<(), String> {
        let target_path = self.base_path.join(key);

        if !target_path.exists() {
            return Err(format!("File not found: {}", target_path.display()));
        }

        fs::remove_file(&target_path)
            .await
            .map_err(|e| e.to_string())?;

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

        // 创建测试文件
        let test_file = temp_dir.path().join("test.txt");
        let mut file = File::create(&test_file).await.unwrap();
        file.write_all(b"test content").await.unwrap();

        // 测试上传
        let result = storage.upload(&test_file, "uploads/").await;
        assert!(result.is_ok());

        // 检查文件是否被复制
        let target_file = temp_dir.path().join("test.txt");
        assert!(target_file.exists());
    }

    #[tokio::test]
    async fn test_local_storage_list() {
        let temp_dir = tempdir().unwrap();
        let storage = LocalStorage::new(temp_dir.path().to_str().unwrap()).await;

        // 创建测试文件
        let test_file = temp_dir.path().join("test.txt");
        let mut file = File::create(&test_file).await.unwrap();
        file.write_all(b"test content").await.unwrap();

        // 测试列表
        let items = storage.list("*.txt").await.unwrap();
        println!("items {:?}", items);
        assert!(!items.is_empty());
        assert_eq!(items[0].key, "test.txt");
    }

    #[tokio::test]
    async fn test_local_storage_delete() {
        let temp_dir = tempdir().unwrap();
        let storage = LocalStorage::new(temp_dir.path().to_str().unwrap()).await;

        // 创建测试文件
        let test_file = temp_dir.path().join("test.txt");
        let mut file = File::create(&test_file).await.unwrap();
        file.write_all(b"test content").await.unwrap();

        // 测试删除
        let result = storage.delete("test.txt").await;
        assert!(result.is_ok());
        assert!(!test_file.exists());
    }
}
