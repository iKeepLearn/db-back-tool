use crate::config::AppConfig;
use crate::database::Database;
use crate::storage::Storage;
use crate::{compression, utils};
use anyhow::Result;
use std::path::PathBuf;
use tracing::{error, info};

pub async fn backup_database(
    db: &dyn Database,
    database_name: &str,
    back_dir: &PathBuf,
    password: &str,
) -> Result<()> {
    // 1. 备份数据库
    let backup_file = db.backup(database_name, back_dir).await?;
    info!("Database backup created: {:?}", backup_file);

    // 2. 压缩并加密
    let compressed_file = compression::compress_and_encrypt(&backup_file, password).await?;
    info!("Backup compressed: {:?}", compressed_file);

    // 3. 删除原始SQL文件
    if let Err(e) = tokio::fs::remove_file(&backup_file).await {
        error!("Failed to remove temporary SQL file: {}", e);
    }

    info!(
        "Backup completed successfully for database: {}",
        database_name
    );
    Ok(())
}

pub async fn upload_to_cos(
    file: Option<String>,
    all: bool,
    config: &AppConfig,
    storage: &dyn Storage,
) -> Result<()> {
    if let Some(file_path) = file {
        // 上传单个文件
        let path = PathBuf::from(&file_path);
        if !path.exists() {
            anyhow::bail!("File not found: {}", file_path);
        }
        storage
            .upload(&path, &config.cos_path)
            .await
            .map_err(anyhow::Error::msg)?;
        info!("File uploaded successfully: {}", file_path);
    } else if all {
        // 上传所有备份文件
        utils::upload_all_backups(&config.get_backup_dir(), storage, &config.cos_path)
            .await
            .map_err(anyhow::Error::msg)?;
        info!("All backups uploaded successfully");
    } else {
        anyhow::bail!("Please specify either --file or --all flag");
    }

    Ok(())
}

pub async fn delete_from_cos(key: Option<String>, all: bool, storage: &dyn Storage) -> Result<()> {
    if let Some(key_str) = key {
        storage.delete(&key_str).await.map_err(anyhow::Error::msg)?;
        info!("File deleted successfully: {}", key_str);
    } else if all {
        let files = storage.list("db").await.map_err(anyhow::Error::msg)?;

        let yesterday_files: Vec<_> = files
            .clone()
            .into_iter()
            .filter(|item| utils::is_yesterday_before(item.last_modified) && item.size > 0)
            .collect();
        for entry in yesterday_files {
            storage
                .delete(&entry.key)
                .await
                .map_err(anyhow::Error::msg)?;
        }

        info!("yesterday before backups delete successfully");
    } else {
        anyhow::bail!("Please specify either --key or --all flag");
    }

    Ok(())
}
