use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tabled::Tabled;

pub mod tencent_cos;

#[async_trait::async_trait]
pub trait Storage {
    async fn upload(&self, file_path: &PathBuf, cos_path: &str) -> Result<(), String>;
    async fn list(&self, key: &str) -> Result<Vec<CosItem>, String>;
    async fn delete(&self, backup_name: &str) -> Result<(), String>;
}

#[derive(Debug, Serialize, Deserialize, Tabled, Clone)]
pub struct CosItem {
    #[tabled(rename = "文件路径")]
    pub key: String,
    #[tabled(rename = "修改时间")]
    pub last_modified: DateTime<Utc>,
    #[tabled(skip)]
    pub size: u64,
    #[tabled(rename = "大小")]
    pub human_size: String,
}
