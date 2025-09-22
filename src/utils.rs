use crate::storage::{CosItem, Storage};
use anyhow::Result;
use chrono::{DateTime, Utc};
use glob::glob;
use quickxml_to_serde::{xml_string_to_json, Config as QuickXmlConfig};
use std::path::PathBuf;
use tabled::Table;
use tracing::{error, info};

pub fn resolve_path(path_str: &str) -> Result<PathBuf, String> {
    let resolved_path = if path_str.starts_with("~") {
        let expanded_str = shellexpand::tilde(path_str);
        PathBuf::from(expanded_str.to_string())
    } else {
        PathBuf::from(path_str)
    };

    if resolved_path.exists() {
        std::fs::canonicalize(&resolved_path)
            .map_err(|e| format!("Could not canonicalize path: {}", e))
    } else {
        Ok(resolved_path)
    }
}

pub fn convert_xml_to_json(xml: &str) -> Result<serde_json::Value> {
    let config = QuickXmlConfig::new_with_defaults();
    let json = xml_string_to_json(xml.to_string(), &config)?;
    Ok(json)
}

pub fn is_yesterday_before(date: DateTime<Utc>) -> bool {
    let today = Utc::now().date_naive();
    let yesterday = today.pred_opt();
    match yesterday {
        Some(yest) => date.date_naive() < yest,
        None => false,
    }
}

pub async fn upload_all_backups<S: Storage>(
    backup_dir: &PathBuf,
    storage: &S,
    cos_path: &str,
) -> Result<(), String> {
    let pattern = backup_dir.join("*.7z").to_string_lossy().to_string();

    let files = glob(&pattern).map_err(|e| e.to_string())?;

    for entry in files {
        match entry {
            Ok(path) => {
                info!("Uploading file: {:?}", path);
                storage.upload(&path, cos_path).await?;
            }
            Err(e) => {
                error!("Error reading file: {}", e);
                return Err(format!("Error reading file: {}", e));
            }
        }
    }
    Ok(())
}

pub async fn cleanup_old_backups(backup_dir: &PathBuf) -> Result<()> {
    let pattern = backup_dir.join("*.7z").to_string_lossy().to_string();

    let files = glob(&pattern)?;

    for entry in files {
        match entry {
            Ok(path) => {
                info!("Remove file: {:?}", &path);
                if let Err(e) = tokio::fs::remove_file(&path).await {
                    error!(
                        "Failed to remove old backup {}: {}",
                        &path.display().to_string(),
                        e
                    );
                } else {
                    info!("Removed old backup: {}", &path.display().to_string());
                }
            }
            Err(e) => {
                error!("Error reading file: {}", e);
            }
        }
    }

    Ok(())
}

pub fn list_backups(files: Vec<CosItem>) -> Result<()> {
    let table = Table::new(&files).to_string();
    println!("=== COS 文件列表 ===");
    println!("{}", table);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_resolve_path_existing() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("testfile.txt");
        File::create(&file_path).unwrap();

        let resolved = resolve_path(file_path.to_str().unwrap()).unwrap();
        assert!(resolved.is_absolute());
        assert!(resolved.ends_with("testfile.txt"));
    }

    #[test]
    fn test_resolve_path_non_existing() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("not_exist.txt");

        let resolved = resolve_path(file_path.to_str().unwrap()).unwrap();
        assert_eq!(resolved, file_path);
    }

    #[test]
    fn test_resolve_path_with_tilde() {
        let home = env::var("HOME").unwrap();
        let test_path = "~/testfile";
        let resolved = resolve_path(test_path).unwrap();
        assert!(resolved.starts_with(&home));
        assert!(resolved.ends_with("testfile"));
    }
}
