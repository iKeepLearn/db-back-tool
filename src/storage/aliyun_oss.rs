use super::CosItem;
use crate::storage::Storage;
use crate::{config::AliyunOssConfig, utils::convert_xml_to_json};
use aliyun_oss_rs::{Error as OssError, OssBucket, OssClient};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::vec;
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
    pub client: OssBucket,
}

#[async_trait::async_trait]
impl Storage for AliyunOss {
    async fn upload(&self, file_path: &Path, cos_path: &str) -> Result<(), String> {
        let file_name = file_path.file_name().unwrap().to_string_lossy();
        let oss_path_full = format!("{}{}", cos_path, file_name);
        let object = self.client.object(&oss_path_full);

        let res = object.put_object().send_file(&file_name).await;

        if res.is_ok() {
            info!("Successfully uploaded: {}", file_name);
            Ok(())
        } else {
            Err(format!(
                "COS upload failed: {}",
                file_path.to_string_lossy()
            ))
        }
    }

    async fn list(&self, key: &str) -> Result<Vec<CosItem>, String> {
        let res = self
            .client
            .list_objects()
            .set_prefix(key)
            .set_max_keys(200)
            .send()
            .await;
        match res {
            Ok(response) => {
                println!("aliyun oss response: {:?}", response);
                let contents = response.contents;
                match contents {
                    Some(value) => {
                        println!("Total items: {:?}", value);
                        let items: Vec<CosItem> = value
                            .iter()
                            .filter_map(|item| {
                                // Try to parse the last_modified string into DateTime<Utc>
                                match DateTime::parse_from_rfc3339(&item.last_modified)
                                    .map(|dt| dt.with_timezone(&Utc))
                                {
                                    Ok(last_modified) => Some(AliyunOssItem {
                                        key: item.key.clone(),
                                        last_modified,
                                        size: item.size,
                                    }),
                                    Err(e) => {
                                        info!(
                                            "Failed to parse last_modified: {} ({})",
                                            item.last_modified, e
                                        );
                                        None
                                    }
                                }
                            })
                            .map(CosItem::from)
                            .collect();
                        Ok(items)
                    }
                    None => Ok(vec![]),
                }
            }
            Err(e) => match e {
                OssError::OssInvalidResponse(Some(value)) => {
                    let string = String::from_utf8(value.to_vec());
                    match string {
                        Ok(s) => {
                            let json = convert_xml_to_json(&s)
                                .map_err(|e| format!("convert_xml_to_json error: {}", e))?;
                            let contents = &json["ListBucketResult"]["Contents"];

                            if contents.is_array() {
                                let contents = &contents.as_array().ok_or_else(|| {
                                    "Expected 'Contents' to be an array".to_string()
                                })?;
                                let mut result: Vec<AliyunOssItem> = contents
                                    .iter()
                                    .map(|item| {
                                        serde_json::from_value(item.clone())
                                            .map_err(|e| format!("serde_json error: {}", e))
                                    })
                                    .collect::<Result<_, _>>()?;
                                result.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));
                                return Ok(result.into_iter().map(|item| item.into()).collect());
                            }

                            if contents.is_object() {
                                return Ok(vec![serde_json::from_value(contents.clone())
                                    .map_err(|e| format!("serde_json error: {}", e))
                                    .map(|item: AliyunOssItem| item.into())?]);
                            }

                            Err(format!("failed to parse result {}", s))
                        }
                        Err(e) => Err(format!("SUCCESS (but failed to decode UTF-8): {}", e)),
                    }
                }
                _ => {
                    println!("aliyun oss error response: {:?}", e);
                    Err(format!("COS list failed: {}", e))
                }
            },
        }
    }

    async fn delete(&self, key: &str) -> Result<(), String> {
        let object = self.client.object(key);

        let res = object.del_object().send().await;

        if res.is_ok() {
            info!("Successfully Deleted: {}", &key);
            Ok(())
        } else {
            Err(format!("delete from aliyun oss failed: {}", key))
        }
    }
}

impl AliyunOss {
    pub fn new(config: &AliyunOssConfig) -> Self {
        let client = OssClient::new(&config.secret_id, &config.secret_key);
        let bucket = client.bucket(&config.bucket, &config.end_point);
        AliyunOss { client: bucket }
    }
}
