use super::Database;
use crate::config::MySqlConfig;
use chrono::Utc;
use serde::Deserialize;
use std::ops::Deref;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Deserialize, Clone)]
pub struct MySql(MySqlConfig);

impl Deref for MySql {
    type Target = MySqlConfig;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait::async_trait]
impl Database for MySql {
    async fn backup(
        &self,
        database_name: &str,
        backup_dir: &PathBuf,
    ) -> anyhow::Result<std::path::PathBuf> {
        let backup_filename = format!(
            "{}_{}.sql",
            database_name,
            Utc::now().format("%Y%m%d_%H%M%S")
        );
        let backup_path = backup_dir.join(&backup_filename);

        // 使用mysqldump进行备份
        let mut cmd = tokio::process::Command::new("mysqldump");

        cmd.arg("-h")
            .arg(&self.host)
            .arg("-P")
            .arg(self.port.to_string())
            .arg("-u")
            .arg(&self.username)
            .arg(format!("-p{}", &self.password))
            .arg(database_name);

        let output = cmd.output().await?;

        if !output.status.success() {
            anyhow::bail!(
                "mysqldump failed for database {}: {}",
                database_name,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        // 确保备份目录存在
        tokio::fs::create_dir_all(&backup_dir).await?;

        let mut file = File::create(&backup_path).await?;
        file.write_all(&output.stdout).await?;
        file.flush().await?;

        Ok(backup_path)
    }
}

impl MySql {
    pub fn new(config: &MySqlConfig) -> Self {
        MySql(MySqlConfig {
            host: config.host.clone(),
            port: config.port,
            username: config.username.clone(),
            password: config.password.clone(),
        })
    }
}
