use super::CosItem;
use crate::config::TencentCosConfig;
use crate::{storage::Storage, utils::convert_xml_to_json};
use chrono::{DateTime, Utc};
use humansize::{format_size, DECIMAL};
use qcos::client::Client;
use qcos::request::ErrNo;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
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
            human_size: format_size(item.size, DECIMAL),
        }
    }
}

pub struct TencentCos {
    pub client: Client,
}

#[async_trait::async_trait]
impl Storage for TencentCos {
    async fn upload(&self, file_path: &PathBuf, cos_path: &str) -> Result<(), String> {
        let file_name = file_path.file_name().unwrap().to_string_lossy();
        let cos_path_full = format!("{}{}", cos_path, file_name);

        let res = self
            .client
            .put_object(file_path, &cos_path_full, None, None)
            .await;
        if res.error_no == ErrNo::SUCCESS {
            info!("Successfully uploaded: {}", file_name);
            Ok(())
        } else {
            Err(format!("COS upload failed: {}", res.error_message))
        }
    }

    async fn list(&self, key: &str) -> Result<Vec<CosItem>, String> {
        let res = self.client.list_objects(key.into(), "", "", "", 0).await;
        if res.error_no == ErrNo::SUCCESS {
            match String::from_utf8(res.result) {
                Ok(s) => {
                    let json = convert_xml_to_json(&s)
                        .map_err(|e| format!("convert_xml_to_json error: {}", e))?;
                    let contents = json["ListBucketResult"]["Contents"]
                        .as_array()
                        .ok_or_else(|| "Expected 'Contents' to be an array".to_string())?;
                    let mut result: Vec<TencentCosItem> = contents
                        .iter()
                        .map(|item| {
                            serde_json::from_value(item.clone())
                                .map_err(|e| format!("serde_json error: {}", e))
                        })
                        .collect::<Result<_, _>>()?;
                    result.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));
                    Ok(result.into_iter().map(|item| item.into()).collect())
                }
                Err(e) => Err(format!("SUCCESS (but failed to decode UTF-8): {}", e)),
            }
        } else {
            Err(format!("COS lists failed: {:?}", res.result))
        }
    }

    async fn delete(&self, key: &str) -> Result<(), String> {
        let res = self.client.delete_object(key).await;
        if res.error_no == ErrNo::SUCCESS {
            info!("Successfully deleted: {}", key);
            Ok(())
        } else {
            Err(format!("COS delete failed: {}", res.error_message))
        }
    }
}

impl TencentCos {
    pub fn new(config: &TencentCosConfig) -> Self {
        let client = Client::new(
            config.secret_id.clone(),
            config.secret_key.clone(),
            config.bucket.clone(),
            config.region.clone(),
        );
        TencentCos { client }
    }
}
