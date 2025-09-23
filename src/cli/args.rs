// src/cli.rs
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "backupdbtool")]
#[command(about = "PostgreSQL\\MySql database backup tool with COS upload", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
    /// Specific file for configuration
    #[arg(short, long)]
    pub config: String,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Backup a specific database
    Backup {
        /// Database name to backup
        database_name: String,
    },

    /// Upload all backup files to COS
    Upload {
        /// Specific file to upload (optional)
        #[arg(short, long)]
        file: Option<String>,

        /// Upload all files in backup directory
        #[arg(short, long, default_value_t = false)]
        all: bool,
    },

    /// Delete yesterday before files from COS
    Delete {
        /// Specific file to delete (optional)
        #[arg(short, long)]
        key: Option<String>,

        /// Delete all yesterday before files
        #[arg(short, long, default_value_t = false)]
        all: bool,
    },

    /// List available backups
    List,

    /// Show tool version
    Version,
}
