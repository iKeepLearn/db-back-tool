use super::CosItem;
use crate::config::TencentCosConfig;
use crate::storage::Storage;
use chrono::{DateTime, Utc};
use cos_rust_sdk::{BucketClient, Config, CosClient, ListObjectsV2Options, ObjectClient};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::info;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TencentCosItem {
    #[serde(rename = "Key")]
    pub key: String,
    #[serde(rename = "LastModified")]
    pub last_modified: DateTime<Utc>,
    #[serde(rename = "Size")]
    pub size: u64,
}

impl From<TencentCosItem> for CosItem {
    fn from(item: TencentCosItem) -> Self {
        CosItem {
            key: item.key,
            last_modified: item.last_modified,
            size: item.size,
        }
    }
}

pub struct TencentCos {
    pub client: CosClient,
}

#[async_trait::async_trait]
impl Storage for TencentCos {
    async fn upload(&self, file_path: &Path, cos_path: &str) -> Result<(), String> {
        let file_name = file_path.file_name().unwrap().to_string_lossy();
        let cos_path_full = format!("{}{}", cos_path, file_name);
        let object_client = ObjectClient::new(self.client.clone());

        match object_client
            .put_object_from_file(&cos_path_full, &PathBuf::from(file_path), None)
            .await
        {
            Ok(_) => {
                info!("Successfully uploaded: {}", file_name);
                Ok(())
            }
            Err(e) => Err(format!("COS upload failed: {}", e)),
        }
    }

    async fn list(&self, key: &str) -> Result<Vec<CosItem>, String> {
        let bucket_client = BucketClient::new(self.client.clone());
        let list_options = ListObjectsV2Options {
            prefix: Some(key.to_string()),
            ..Default::default()
        };
        match bucket_client.list_objects_v2(Some(list_options)).await {
            Ok(response) => {
                if response.contents.is_empty() {
                    return Ok(vec![]);
                } else {
                    let result = response
                        .contents
                        .into_iter()
                        .map(|item| {
                            match DateTime::parse_from_rfc3339(&item.last_modified)
                                .map(|dt| dt.with_timezone(&Utc))
                            {
                                Ok(last_modified) => CosItem {
                                    key: item.key,
                                    last_modified,
                                    size: item.size,
                                },
                                Err(e) => {
                                    info!(
                                        "Failed to parse last_modified: {} ({})",
                                        item.last_modified, e
                                    );
                                    CosItem {
                                        key: item.key,
                                        last_modified: Utc::now(),
                                        size: item.size,
                                    }
                                }
                            }
                        })
                        .collect();
                    return Ok(result);
                }
            }
            Err(e) => {
                info!("list cos objects failed: {}", e);
                Err(e.to_string())
            }
        }
    }

    async fn delete(&self, key: &str) -> Result<(), String> {
        let object_client = ObjectClient::new(self.client.clone());
        match object_client.delete_object(key).await {
            Ok(_) => {
                info!("Successfully deleted: {}", key);
                Ok(())
            }
            Err(e) => Err(format!("COS delete failed: {}", e)),
        }
    }
}

impl TencentCos {
    pub fn new(config: &TencentCosConfig) -> Self {
        // 创建配置
        let config = Config::new(
            &config.secret_id,
            &config.secret_key,
            &config.region,
            &config.bucket,
        )
        .with_timeout(Duration::from_secs(30))
        .with_https(true);

        // 创建客户端
        let client = CosClient::new(config).expect("init cos client failed");
        TencentCos { client }
    }
}
