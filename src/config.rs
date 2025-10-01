use crate::database::Database;
use crate::database::{mysql::MySql, postgresql::PostgreSql};
use crate::storage::aliyun_oss::AliyunOss;
use crate::storage::local_storage::LocalStorage;
use crate::storage::s3_compatible::S3Oss;
use crate::storage::tencent_cos::TencentCos;
use crate::storage::Storage;
use config::{Config, ConfigError, File};
use serde::Deserialize;
use std::path::PathBuf;
use tokio::fs::create_dir_all;

#[derive(Debug, Deserialize, Clone)]
pub struct AllConfig {
    pub app: AppConfig,
    pub tencent_cos: TencentCosConfig,
    pub postgresql: PostgreSqlConfig,
    pub mysql: MySqlConfig,
    pub aliyun_oss: AliyunOssConfig,
    pub s3: S3OssConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub backup_dir: PathBuf,
    pub db_type: DbType,
    pub cos_provider: CosProvider,
    pub cos_path: String,
    pub compress_password: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TencentCosConfig {
    pub secret_id: String,
    pub secret_key: String,
    pub region: String,
    pub bucket: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AliyunOssConfig {
    pub secret_id: String,
    pub secret_key: String,
    pub end_point: String,
    pub bucket: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct S3OssConfig {
    pub secret_id: String,
    pub secret_key: String,
    pub end_point: Option<String>,
    pub bucket: String,
    pub region: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PostgreSqlConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MySqlConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub enum DbType {
    #[serde(rename = "postgresql")]
    Postgresql,
    #[serde(rename = "mysql")]
    MySql,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub enum CosProvider {
    #[serde(rename = "tencent_cos")]
    TencentCos,
    #[serde(rename = "aliyun_oss")]
    AliyunOss,
    #[serde(rename = "local")]
    LocalStorage,
    #[serde(rename = "s3")]
    S3,
}

impl Default for AppConfig {
    fn default() -> Self {
        let backup_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join(".dbbackup");

        AppConfig {
            backup_dir,
            db_type: DbType::Postgresql,
            cos_provider: CosProvider::TencentCos,
            cos_path: "db/".into(),
            compress_password: "dbbackuppassword".into(),
        }
    }
}

impl AppConfig {
    pub async fn confirm_backup_dir(&self) {
        let home_dir: PathBuf = AppConfig::default().backup_dir;
        let result = create_dir_all(&self.backup_dir).await;
        if result.is_err() {
            create_dir_all(home_dir).await.unwrap();
        }
    }

    pub fn get_backup_dir(&self) -> PathBuf {
        self.backup_dir.clone()
    }

    pub fn database(&self, config: &AllConfig) -> Box<dyn Database> {
        match self.db_type {
            DbType::Postgresql => {
                let postgresql = PostgreSql::new(&config.postgresql);
                Box::new(postgresql)
            }
            DbType::MySql => {
                let mysql = MySql::new(&config.mysql);
                Box::new(mysql)
            }
        }
    }
    pub async fn storage(&self, config: &AllConfig) -> Box<dyn Storage> {
        match self.cos_provider {
            CosProvider::TencentCos => {
                let config = &config.tencent_cos;
                let storage = TencentCos::new(config);
                Box::new(storage)
            }
            CosProvider::AliyunOss => {
                let config = &config.aliyun_oss;
                let storage = AliyunOss::new(config);
                Box::new(storage)
            }
            CosProvider::LocalStorage => {
                let path = &config.app.get_backup_dir();
                let storage = LocalStorage::new(path.to_str().unwrap()).await;
                Box::new(storage)
            }
            CosProvider::S3 => {
                let config = &config.s3;
                let storage = S3Oss::new(config);
                Box::new(storage)
            }
        }
    }
}

pub fn get_all_config(config_path: &str) -> anyhow::Result<AllConfig, ConfigError> {
    let config_builder = Config::builder()
        // 加载配置文件
        .add_source(File::with_name(config_path))
        .build()?;

    let config = config_builder.try_deserialize()?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File as StdFile;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_get_all_config() {
        // 创建一个临时目录
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_config.toml");

        // 构造一个配置文件内容
        let config_content = r#"
            [app]
            backup_dir = "/tmp/dbbackup"
            db_type = "postgresql"
            cos_provider = "tencent_cos"
            cos_path = "db/"
            compress_password = "testpassword"

            [tencent_cos]
            secret_id = "testid"
            secret_key = "testkey"
            region = "ap-guangzhou"
            bucket = "testbucket"

            [postgresql]
            host = "localhost"
            port = 5432
            username = "user"
            password = "pass"

            [mysql]
            host = "localhost"
            port = 3306
            username = "user"
            password = "pass"

            [aliyun_oss]
            secret_id = "testid"
            secret_key = "testkey"
            end_point = "ap-guangzhou"
            bucket = "testbucket"
        "#;

        // 写入临时配置文件
        let mut file = StdFile::create(&file_path).unwrap();
        file.write_all(config_content.as_bytes()).unwrap();

        // 调用get_all_config
        let config = get_all_config(file_path.to_str().unwrap()).unwrap();

        // 断言配置内容
        assert_eq!(config.app.backup_dir, PathBuf::from("/tmp/dbbackup"));
        assert_eq!(config.app.db_type, DbType::Postgresql);
        assert_eq!(config.app.cos_provider, CosProvider::TencentCos);
        assert_eq!(config.app.cos_path, "db/");
        assert_eq!(config.app.compress_password, "testpassword");

        assert_eq!(config.tencent_cos.secret_id, "testid");
        assert_eq!(config.tencent_cos.secret_key, "testkey");
        assert_eq!(config.tencent_cos.region, "ap-guangzhou");
        assert_eq!(config.tencent_cos.bucket, "testbucket");

        assert_eq!(config.postgresql.host, "localhost");
        assert_eq!(config.postgresql.port, 5432);
        assert_eq!(config.postgresql.username, "user");
        assert_eq!(config.postgresql.password, "pass");
    }
}
