// src/main.rs

use anyhow::Result;
use backupdbtool::cli::{Cli, Commands};
use backupdbtool::config::{get_all_config, AllConfig};
use clap::Parser;
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
    println!("all config: {:?}", config);
    println!("app config: {:?}", app_config);
    Ok(())
}
