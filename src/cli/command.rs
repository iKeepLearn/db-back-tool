use crate::config::{AllConfig, AppConfig};
use crate::crypt::aes::{
    EncryptedPackage, decrypt_data, encrypt_data, generate_key_from_password, generate_salt,
};
use crate::database::Database;
use crate::notify::Notify;
use crate::notify::webhook::{WebHookNotify, WebHookSendData};
use crate::storage::Storage;
use crate::{compression,utils};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{error, info};

pub async fn backup_database(
    db: &dyn Database,
    database_name: &str,
    back_dir: &Path,
    password: &str,
    notify: Option<WebHookNotify>,
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

    if let Some(notify) = notify {
        let message = format!("数据库 {} 备份成功", database_name);
        let data = WebHookSendData::new("备份进度", message);
        notify.send(data).await?;
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
    storage: Arc<dyn Storage>,
    notify: Option<WebHookNotify>,
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
        if let Some(notify) = notify {
            let message = format!("{} 上传成功", file_path);
            let data = WebHookSendData::new("备份进度", message);
            notify.send(data).await?;
        }
        info!("File uploaded successfully: {}", file_path);
    } else if all {
        // 上传所有备份文件
        utils::upload_all_backups(&config.get_backup_dir(), storage, &config.cos_path)
            .await
            .map_err(anyhow::Error::msg)?;
        if let Some(notify) = notify {
            let data = WebHookSendData::new("备份进度", "所有备份文件上传成功");
            notify.send(data).await?;
        }
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

pub fn encrypt_yaml_file(source: &PathBuf, destination: &PathBuf, password: &str) -> Result<()> {
    // Read the source yaml file
    let toml_content = fs::read_to_string(source)
        .with_context(|| format!("Failed to read source file: {}", source.display()))?;

    // Parse yaml to validate it's valid
    let _config: AllConfig =
        serde_yml::from_str(&toml_content).with_context(|| "Invalid YAML format in source file")?;

    // Generate a salt and derive a key from the password
    let salt = generate_salt();
    let key = generate_key_from_password(password.as_bytes(), &salt)?;

    // Encrypt the data
    let encrypted_data = encrypt_data(toml_content.as_bytes(), &key)?;

    // Prepare the encrypted package with salt
    let encrypted_package = EncryptedPackage {
        salt: salt.to_vec(),
        ciphertext: encrypted_data,
    };

    // Serialize and save
    let serialized = serde_json::to_string(&encrypted_package)
        .context("Failed to serialize encrypted package")?;

    fs::write(destination, serialized).with_context(|| {
        format!(
            "Failed to write to destination file: {}",
            destination.display()
        )
    })?;

    info!(
        "File encrypted successfully: {} -> {}",
        source.display(),
        destination.display()
    );

    Ok(())
}

pub fn decrypt_yaml_file(encrypted_file: &PathBuf, password: &str) -> Result<AllConfig> {
    // Read the encrypted file
    let encrypted_content = fs::read_to_string(encrypted_file).with_context(|| {
        format!(
            "Failed to read encrypted file: {}",
            encrypted_file.display()
        )
    })?;

    // Parse the encrypted package
    let encrypted_package: EncryptedPackage =
        serde_json::from_str(&encrypted_content).context("Invalid encrypted file format")?;

    // Derive the key from the password and salt
    let key = generate_key_from_password(password.as_bytes(), &encrypted_package.salt)?;

    // Decrypt the data
    let decrypted_data = decrypt_data(&encrypted_package.ciphertext, &key)?;

    // Parse the decrypted yaml to validate it's valid
    let decrypted_str =
        String::from_utf8(decrypted_data).context("Decrypted data is not valid UTF-8")?;

    let config: AllConfig =
        serde_yml::from_str(&decrypted_str).context("Decrypted data is not valid YAML format")?;

    Ok(config)
}
