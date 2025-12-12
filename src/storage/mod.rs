use anyhow::Result;
use chrono::{DateTime, Utc};
use humansize::{DECIMAL, format_size};
use serde::{Deserialize, Serialize};
use std::borrow::Cow::{self, Borrowed};
use std::cmp::Ordering;
use std::path::Path;
use tabled::Tabled;

pub mod aliyun_oss;
pub mod local_storage;
pub mod s3_compatible;
pub mod tencent_cos;

#[async_trait::async_trait]
pub trait Storage: Send + Sync {
    async fn upload(&self, file_path: &Path, cos_path: &str) -> Result<(), String>;
    async fn list(&self, key: &str) -> Result<Vec<CosItem>, String>;
    async fn delete(&self, backup_name: &str) -> Result<(), String>;
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CosItem {
    pub key: String,
    pub last_modified: DateTime<Utc>,
    pub size: u64,
}

impl PartialEq for CosItem {
    fn eq(&self, other: &Self) -> bool {
        self.last_modified == other.last_modified
    }
}

impl Eq for CosItem {}

impl PartialOrd for CosItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CosItem {
    fn cmp(&self, other: &Self) -> Ordering {
        self.last_modified.cmp(&other.last_modified)
    }
}

impl Tabled for CosItem {
    const LENGTH: usize = 3;
    fn headers() -> Vec<Cow<'static, str>> {
        vec![Borrowed("文件路径"), Borrowed("修改时间"), Borrowed("大小")]
    }
    fn fields(&self) -> Vec<Cow<'_, str>> {
        let human_size = format_size(self.size, DECIMAL);
        let last_modified = self.last_modified.format("%Y-%m-%d %H:%M").to_string();
        vec![
            self.key.clone().into(),
            last_modified.into(),
            human_size.into(),
        ]
    }
}
