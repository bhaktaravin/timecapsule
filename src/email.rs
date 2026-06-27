use std::sync::Arc;

use anyhow::{Context, Result};
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

#[derive(Clone)]
pub struct EmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub from_address: String,
    pub app_base_url: String,
}

impl EmailConfig {
    pub fn from_env() -> Result<Option<Self>> {
        let smtp_host = match std::env::var("SMTP_HOST") {
            Ok(value) if !value.trim().is_empty() => value,
            _ => return Ok(None),
        };

        Ok(Some(Self {
            smtp_host,
            smtp_port: std::env::var("SMTP_PORT")
                .unwrap_or_else(|_| "587".to_string())
                .parse()
                .context("SMTP_PORT must be a valid u16")?,
            smtp_username: std::env::var("SMTP_USERNAME")
                .context("SMTP_USERNAME must be set when SMTP_HOST is set")?,
            smtp_password: std::env::var("SMTP_PASSWORD")
                .context("SMTP_PASSWORD must be set when SMTP_HOST is set")?,
            from_address: std::env::var("SMTP_FROM")
                .context("SMTP_FROM must be set when SMTP_HOST is set")?,
            app_base_url: std::env::var("APP_BASE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:3000".to_string())
                .trim_end_matches('/')
                .to_string(),
        }))
    }

    pub fn unlock_url(&self, token: &str) -> String {
        format!("{}/unlock/?token={}", self.app_base_url, token)
    }
}

pub async fn send_ready_notification(
    config: &EmailConfig,
    recipient: &str,
    unlock_url: &str,
) -> Result<()> {
    let body = format!(
        "Someone sealed a message for you in Timecapsule.\n\n\
         Your message is ready to open:\n\
         {unlock_url}\n\n\
         This link works once. Open it when you are ready."
    );

    let message = Message::builder()
        .from(config.from_address.parse().context("invalid SMTP_FROM address")?)
        .to(recipient.parse().context("invalid recipient email address")?)
        .subject("Your timecapsule message is ready")
        .header(ContentType::TEXT_PLAIN)
        .body(body)?;

    let credentials = Credentials::new(
        config.smtp_username.clone(),
        config.smtp_password.clone(),
    );

    let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_host)?
        .port(config.smtp_port)
        .credentials(credentials)
        .build();

    mailer.send(message).await?;

    Ok(())
}

pub type SharedEmailConfig = Arc<Option<EmailConfig>>;
