use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("YAML serialization error: {0}")]
    Yaml(#[from] serde_yml::Error),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Decryption error: {0}")]
    Decryption(String),

    #[error("Database backup error: {0}")]
    DatabaseBackup(String),

    #[error("Compression error: {0}")]
    Compression(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Storage upload error for {path}: {message}")]
    StorageUpload { path: PathBuf, message: String },

    #[error("Storage list error: {0}")]
    StorageList(String),

    #[error("Storage delete error for {key}: {message}")]
    StorageDelete { key: String, message: String },

    #[error("Notification error: {0}")]
    Notification(String),

    #[error("Path resolution error: {0}")]
    PathResolution(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Command execution failed: {0}")]
    CommandExecution(String),

    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Operation cancelled or no action taken")]
    Cancelled,
}

pub type Result<T> = std::result::Result<T, Error>;
