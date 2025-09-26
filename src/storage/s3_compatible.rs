use super::CosItem;
use crate::config::S3OssConfig;
use crate::storage::Storage;
use chrono::{DateTime, Utc};
use humansize::{format_size, DECIMAL};
use s3::{creds::Credentials, Bucket, Region};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::info;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct S3OssItem {
    #[serde(rename = "Key")]
    pub key: String,
    #[serde(rename = "LastModified")]
    pub last_modified: DateTime<Utc>,
    #[serde(rename = "Size")]
    pub size: u64,
}

impl From<S3OssItem> for CosItem {
    fn from(item: S3OssItem) -> Self {
        CosItem {
            key: item.key,
            last_modified: item.last_modified,
            size: item.size,
            human_size: format_size(item.size, DECIMAL),
        }
    }
}

pub struct S3Oss {
    pub bucket: Box<Bucket>,
    pub bucket_name: String,
}

#[async_trait::async_trait]
impl Storage for S3Oss {
    async fn upload(&self, file_path: &PathBuf, cos_path: &str) -> Result<(), String> {
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

        // 上传到 S3
        let res = self
            .bucket
            .put_object(&s3_key, &content)
            .await
            .map_err(|e| format!("S3 upload failed: {}", e))?;

        if res.status_code() == 200 {
            info!(
                "Successfully uploaded: {} to s3://{}/{}",
                file_name, self.bucket_name, s3_key
            );
            Ok(())
        } else {
            Err(format!(
                "S3 upload failed with HTTP code: {}",
                res.status_code()
            ))
        }
    }

    async fn list(&self, prefix: &str) -> Result<Vec<CosItem>, String> {
        let result = self
            .bucket
            .list(prefix.to_string(), None)
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
                        Ok(last_modified) => Some(S3OssItem {
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
                .map(CosItem::from)
                .collect();

            all_items.extend(items);
        }

        Ok(all_items)
    }

    async fn delete(&self, key: &str) -> Result<(), String> {
        let res = self
            .bucket
            .delete_object(key)
            .await
            .map_err(|e| format!("S3 delete failed: {}", e))?;

        if res.status_code() == 200 || res.status_code() == 204 {
            info!("Successfully deleted: s3://{}/{}", self.bucket_name, key);
            Ok(())
        } else {
            Err(format!(
                "S3 delete failed with HTTP code: {}",
                res.status_code()
            ))
        }
    }
}

impl S3Oss {
    pub fn new(config: &S3OssConfig) -> Self {
        let region_config = match &config.region {
            Some(region) => region,
            None => "",
        };
        // 创建区域配置
        let region = if let Some(endpoint_url) = &config.end_point {
            // 对于 S3 兼容服务（如 MinIO、阿里云OSS等），使用自定义端点:cite[3]:cite[9]
            Region::Custom {
                region: region_config.to_string(),
                endpoint: endpoint_url.to_string(),
            }
        } else {
            // 对于 AWS S3，使用标准区域
            region_config.parse().expect("")
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
        let bucket =
            Bucket::new(&config.bucket, region, credentials).expect("create s3 bucket failed");

        S3Oss {
            bucket,
            bucket_name: config.bucket.to_string(),
        }
    }
}
