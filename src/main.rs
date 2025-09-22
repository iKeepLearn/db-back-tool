// src/main.rs

use anyhow::Result;
use backupdbtool::cli::{Cli, Commands};
use clap::Parser;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
}
