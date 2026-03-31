use super::Notify;
use reqwest::{
    Client,
    header::{AUTHORIZATION, HeaderMap, HeaderValue},
};
use serde::Serialize;
use tracing::{error, info};

pub struct WebHookNotify {
    pub client: reqwest::Client,
    pub url: String,
    pub token: Option<String>,
}

#[derive(Serialize)]
pub struct WebHookSendData {
    pub title: String,
    pub message: String,
}

impl WebHookSendData {
    pub fn new(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
        }
    }
}

impl WebHookNotify {
    pub fn new(url: impl Into<String>, token: Option<String>) -> Self {
        let client = if let Some(t) = token.clone() {
            Client::builder()
                .default_headers({
                    let mut headers = HeaderMap::new();
                    headers.insert(
                        AUTHORIZATION,
                        HeaderValue::from_str(&format!("Bearer {}", t)).unwrap(),
                    );
                    headers
                })
                .build()
                .unwrap()
        } else {
            Client::new()
        };
        WebHookNotify {
            client: client,
            url: url.into(),
            token,
        }
    }
}

#[async_trait::async_trait]
impl Notify for WebHookNotify {
    type SendData = WebHookSendData;
    type Error = reqwest::Error;
    async fn send(&self, data: Self::SendData) -> Result<(), Self::Error> {
        let res = self.client.post(&self.url).json(&data).send().await?;
        if res.status().is_success() {
            info!("Notification sent successfully");
        } else {
            error!("Failed to send notification {:?}", res.text().await);
        }
        Ok(())
    }
}
