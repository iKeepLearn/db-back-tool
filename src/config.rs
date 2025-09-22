use crate::database::postgresql::PostgreSql;
use crate::database::Database;
use crate::storage::tencent_cos::TencentCos;
use crate::storage::Storage;
use crate::utils::resolve_path;
use config::{Config, ConfigError, File};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
pub struct AllConfig {
    pub app: AppConfig,
    pub tencent_cos: TencentCosConfig,
    pub postgresql: PostgreSqlConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub backup_dir: String,
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
pub struct PostgreSqlConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub enum DbType {
    #[serde(rename = "postgresql")]
    Postgresql,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub enum CosProvider {
    #[serde(rename = "tencent_cos")]
    TencentCos,
}

impl Default for AppConfig {
    fn default() -> Self {
        let backup_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join(".dbbackup");

        AppConfig {
            backup_dir: backup_dir.display().to_string(),
            db_type: DbType::Postgresql,
            cos_provider: CosProvider::TencentCos,
            cos_path: "db/".into(),
            compress_password: "dbbackuppassword".into(),
        }
    }
}

impl AppConfig {
    pub fn get_backup_dir(&self) -> PathBuf {
        let path = resolve_path(&self.backup_dir);
        match path {
            Ok(p) => p,
            Err(_) => AppConfig::default().backup_dir.into(),
        }
    }
    pub fn database(&self, config: &AllConfig) -> impl Database {
        match self.db_type {
            DbType::Postgresql => {
                let db = PostgreSql::new(&config.postgresql);
                db
            }
        }
    }
    pub fn storage(&self, config: &AllConfig) -> impl Storage {
        match self.cos_provider {
            CosProvider::TencentCos => {
                let config = &config.tencent_cos;
                let storage = TencentCos::new(config);
                storage
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
        "#;

        // 写入临时配置文件
        let mut file = StdFile::create(&file_path).unwrap();
        file.write_all(config_content.as_bytes()).unwrap();

        // 调用get_all_config
        let config = get_all_config(file_path.to_str().unwrap()).unwrap();

        // 断言配置内容
        assert_eq!(config.app.backup_dir, "/tmp/dbbackup");
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
