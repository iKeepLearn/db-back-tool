// src/compression.rs
use anyhow::Result;
// use sevenz_rust2::{compress_to_path_encrypted, Password};
use std::path::{Path, PathBuf};
use tokio::process::Command;

pub async fn compress_and_encrypt(input_file: &Path, password: &str) -> Result<PathBuf> {
    let output_path = input_file.with_extension("7z");
    // let password = Password::from(password);
    // compress_to_path_encrypted(input_file, &output_path, password)?;

    let mut cmd = Command::new("7z");

    cmd.arg("a")
        .arg("-t7z")
        .arg("-m0=lzma2")
        .arg("-mhe=on") // 启用头部加密
        .arg(format!("-p{}", password)) // 设置密码
        .arg(&output_path)
        .arg(input_file);

    let status = cmd.status().await?;

    if !status.success() {
        anyhow::bail!("7z compression failed");
    }

    Ok(output_path)
}
