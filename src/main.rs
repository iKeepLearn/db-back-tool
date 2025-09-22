// src/main.rs

use anyhow::Result;
use backupdbtool::cli::{Cli, Commands};
use backupdbtool::compression;
use backupdbtool::config::{get_all_config, AllConfig};
use backupdbtool::database::Database;
use clap::Parser;
use std::path::PathBuf;
use tracing::{error, info};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    // 加载配置
    let config = match get_all_config(&cli.config) {
        Ok(config) => config,
        Err(e) => {
            error!("Failed to load config: {}", e);
            anyhow::bail!(e);
        }
    };
    let app_config = &config.app;
    let db = app_config.database(&config);

    match cli.command {
        Commands::Backup { database_name } => {
            info!("Starting backup for database: {}", database_name);
            backup_database(
                db,
                &database_name,
                &app_config.get_backup_dir(),
                &app_config.compress_password,
            )
            .await
        }
        Commands::Upload { file, all } => Ok(()),
        Commands::Delete { key, all } => Ok(()),
        Commands::List => Ok(()),
        Commands::Version => {
            println!("backupdbtool v{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    }
}

async fn backup_database<Db: Database>(
    db: Db,
    database_name: &str,
    back_dir: &PathBuf,
    password: &str,
) -> Result<()> {
    // 1. 备份数据库
    let backup_file = db.backup(database_name, back_dir).await?;
    info!("Database backup created: {:?}", backup_file);

    // 2. 压缩并加密
    let compressed_file = compression::compress_and_encrypt(&backup_file, password).await?;
    info!("Backup compressed: {:?}", compressed_file);

    // 3. 删除原始SQL文件
    if let Err(e) = tokio::fs::remove_file(&backup_file).await {
        error!("Failed to remove temporary SQL file: {}", e);
    }

    info!(
        "Backup completed successfully for database: {}",
        database_name
    );
    Ok(())
}
