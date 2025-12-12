// src/main.rs

use std::process;

use anyhow::Result;
use backupdbtool::cli::args::{Cli, Commands};
use backupdbtool::cli::command::{backup_database, delete_from_cos, upload_to_cos};
use backupdbtool::config::{CosProvider, get_all_config};
use backupdbtool::storage::CosItem;
use backupdbtool::utils::{self, resolve_path};
use clap::Parser;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    // 加载配置
    let mut config = match get_all_config(&cli.config) {
        Ok(config) => config,
        Err(e) => {
            error!("Failed to load config: {}", e);
            anyhow::bail!(e);
        }
    };
    let result = resolve_path(&config.app.backup_dir.to_string_lossy());
    match result {
        Ok(path) => {
            config.app.backup_dir = path;
        }
        Err(_) => {
            error!(
                "please check the backup_dir path: {}",
                &config.app.backup_dir.to_string_lossy()
            );
            process::exit(1);
        }
    }

    let app_config = &config.app;
    let _ = app_config.confirm_backup_dir().await;
    let db = app_config.database(&config);
    let storage = app_config.storage(&config).await;

    match cli.command {
        Commands::Backup { database_name } => {
            info!("Starting backup for database: {}", database_name);
            backup_database(
                db.as_ref(),
                &database_name,
                &app_config.get_backup_dir(),
                &app_config.compress_password,
            )
            .await
        }
        Commands::Upload { file, all } => {
            info!("Starting upload to COS");
            upload_to_cos(file, all, app_config, storage).await
        }
        Commands::Delete { key, all } => {
            info!("Starting delete yesterday before file from  COS");
            utils::cleanup_old_backups(&app_config.get_backup_dir()).await?;
            delete_from_cos(key, all, storage.as_ref()).await
        }
        Commands::List => {
            let key_str = match &config.app.cos_provider {
                CosProvider::LocalStorage => "*.7z",
                _ => app_config.cos_path.as_str(),
            };
            let files: Vec<CosItem> = storage
                .list(key_str)
                .await
                .map_err(|e| anyhow::anyhow!(e))?;
            let _ = utils::list_table(files);
            Ok(())
        }
        Commands::Version => {
            println!("backupdbtool v{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    }
}
