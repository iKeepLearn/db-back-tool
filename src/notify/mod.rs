pub mod webhook;

use anyhow::Result;
use serde::Serialize;

#[async_trait::async_trait]
pub trait Notify: Send + Sync {
    type SendData:Serialize;
    type Error;
    async fn send(&self, data: Self::SendData) -> Result<(), Self::Error>;
}