use super::CosItem;
use crate::config::AliyunOssConfig;
use crate::storage::Storage;
use chrono::{DateTime, Utc};
use s3::{creds::Credentials, Bucket, Region};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::info;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AliyunOssItem {
    #[serde(rename = "Key")]
    pub key: String,
    #[serde(rename = "LastModified")]
    pub last_modified: DateTime<Utc>,
    #[serde(rename = "Size")]
    pub size: u64,
}

impl From<AliyunOssItem> for CosItem {
    fn from(item: AliyunOssItem) -> Self {
        CosItem {
            key: item.key,
            last_modified: item.last_modified,
            size: item.size,
        }
    }
}

pub struct AliyunOss {
    pub client: Box<Bucket>,
}

#[async_trait::async_trait]
impl Storage for AliyunOss {
    async fn upload(&self, file_path: &Path, cos_path: &str) -> Result<(), String> {
        let file_name = file_path
            .file_name()
            .ok_or_else(|| format!("Invalid file path: {}", file_path.display()))?
            .to_string_lossy();

        let s3_key = if cos_path.ends_with('/') {
            format!("{}{}", cos_path, file_name)
        } else {
            format!("{}/{}", cos_path, file_name)
        };

        // 读取文件内容
        let content = std::fs::read(file_path)
            .map_err(|e| format!("Failed to read file {}: {}", file_path.display(), e))?;

        // 上传到 Aliyun Oss
        let res = self
            .client
            .put_object(&s3_key, &content)
            .await
            .map_err(|e| format!("S3 upload failed: {}", e))?;

        if res.status_code() == 200 {
            info!("Successfully uploaded: {}", file_name,);
            Ok(())
        } else {
            Err(format!(
                "Aliyun oss upload failed with HTTP code: {}",
                res.status_code()
            ))
        }
    }

    async fn list(&self, key: &str) -> Result<Vec<CosItem>, String> {
        let result = self
            .client
            .list(key.to_string(), None)
            .await
            .map_err(|e| format!("S3 list failed: {}", e))?;

        let mut all_items = Vec::new();
        for item in result {
            let contents = item.contents;
            let items: Vec<CosItem> = contents
                .into_iter()
                .filter_map(|object| {
                    // Try to parse the last_modified string into DateTime<Utc>
                    match DateTime::parse_from_rfc3339(&object.last_modified)
                        .map(|dt| dt.with_timezone(&Utc))
                    {
                        Ok(last_modified) => Some(CosItem {
                            key: object.key.clone(),
                            last_modified,
                            size: object.size,
                        }),
                        Err(e) => {
                            info!(
                                "Failed to parse last_modified: {} ({})",
                                object.last_modified, e
                            );
                            None
                        }
                    }
                })
                .collect();

            all_items.extend(items);
        }

        Ok(all_items)
    }

    async fn delete(&self, key: &str) -> Result<(), String> {
        let res = self
            .client
            .delete_object(key)
            .await
            .map_err(|e| format!("Aliyun oss delete failed: {}", e))?;

        if res.status_code() == 200 || res.status_code() == 204 {
            info!("Successfully deleted: {}", key);
            Ok(())
        } else {
            Err(format!(
                "Aliyun oss delete failed with HTTP code: {}",
                res.status_code()
            ))
        }
    }
}

impl AliyunOss {
    pub fn new(config: &AliyunOssConfig) -> Self {
        // 创建区域配置
        let region = Region::Custom {
            region: "".to_string(),
            endpoint: config.end_point.to_string(),
        };

        // 创建凭据
        let credentials = Credentials {
            access_key: Some(config.secret_id.to_string()),
            secret_key: Some(config.secret_key.to_string()),
            security_token: None,
            session_token: None,
            expiration: None,
        };

        // 创建存储桶实例
        let bucket = Bucket::new(&config.bucket, region, credentials)
            .expect("create aliyun oss bucket failed");

        AliyunOss { client: bucket }
    }
}
