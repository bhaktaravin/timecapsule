use std::sync::Arc;

use anyhow::{Context, Result};
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use serde::Serialize;

#[derive(Clone)]
pub enum EmailConfig {
    MailtrapApi(MailtrapApiConfig),
    Smtp(SmtpConfig),
}

#[derive(Clone)]
pub struct MailtrapApiConfig {
    pub api_token: String,
    pub from_email: String,
    pub from_name: String,
    pub app_base_url: String,
}

#[derive(Clone)]
pub struct SmtpConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub from_address: String,
    pub app_base_url: String,
}

impl EmailConfig {
    pub fn from_env() -> Result<Option<Self>> {
        if let Some(api_token) = read_mailtrap_api_token() {
            return Ok(Some(Self::MailtrapApi(MailtrapApiConfig {
                api_token,
                from_email: std::env::var("MAILTRAP_FROM_EMAIL")
                    .unwrap_or_else(|_| "hello@demomailtrap.co".to_string()),
                from_name: std::env::var("MAILTRAP_FROM_NAME")
                    .unwrap_or_else(|_| "Timecapsule".to_string()),
                app_base_url: app_base_url_from_env(),
            })));
        }

        let smtp_host = match std::env::var("SMTP_HOST") {
            Ok(value) if !value.trim().is_empty() => value,
            _ => return Ok(None),
        };

        Ok(Some(Self::Smtp(SmtpConfig {
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
            app_base_url: app_base_url_from_env(),
        })))
    }

    pub fn unlock_url(&self, token: &str) -> String {
        let base = match self {
            Self::MailtrapApi(config) => &config.app_base_url,
            Self::Smtp(config) => &config.app_base_url,
        };
        format!("{base}/unlock/?token={token}")
    }
}

pub async fn send_ready_notification(
    config: &EmailConfig,
    recipient: &str,
    unlock_url: &str,
) -> Result<()> {
    let (subject, body) = notification_content(unlock_url);

    match config {
        EmailConfig::MailtrapApi(api_config) => {
            send_via_mailtrap_api(api_config, recipient, &subject, &body).await
        }
        EmailConfig::Smtp(smtp_config) => {
            send_via_smtp(smtp_config, recipient, &subject, &body).await
        }
    }
}

pub type SharedEmailConfig = Arc<Option<EmailConfig>>;

fn read_mailtrap_api_token() -> Option<String> {
    for key in ["MAILTRAP_API_TOKEN", "MAILTRAP_API_KEY"] {
        if let Ok(value) = std::env::var(key) {
            let trimmed = value.trim();
            if !trimmed.is_empty() && !trimmed.starts_with("paste_") {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn app_base_url_from_env() -> String {
    std::env::var("APP_BASE_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:3000".to_string())
        .trim_end_matches('/')
        .to_string()
}

fn notification_content(unlock_url: &str) -> (String, String) {
    let subject = "Your timecapsule message is ready".to_string();
    let body = format!(
        "Someone sealed a message for you in Timecapsule.\n\n\
         Your message is ready to open:\n\
         {unlock_url}\n\n\
         This link works once. Open it when you are ready."
    );
    (subject, body)
}

#[derive(Serialize)]
struct MailtrapAddress<'a> {
    email: &'a str,
    name: &'a str,
}

#[derive(Serialize)]
struct MailtrapRecipient<'a> {
    email: &'a str,
}

#[derive(Serialize)]
struct MailtrapSendRequest<'a> {
    from: MailtrapAddress<'a>,
    to: Vec<MailtrapRecipient<'a>>,
    subject: &'a str,
    text: &'a str,
    category: &'a str,
}

async fn send_via_mailtrap_api(
    config: &MailtrapApiConfig,
    recipient: &str,
    subject: &str,
    body: &str,
) -> Result<()> {
    let payload = MailtrapSendRequest {
        from: MailtrapAddress {
            email: &config.from_email,
            name: &config.from_name,
        },
        to: vec![MailtrapRecipient { email: recipient }],
        subject,
        text: body,
        category: "Timecapsule",
    };

    let client = reqwest::Client::new();
    let response = client
        .post("https://send.api.mailtrap.io/api/send")
        .bearer_auth(&config.api_token)
        .json(&payload)
        .send()
        .await
        .context("failed to call Mailtrap API")?;

    if response.status().is_success() {
        return Ok(());
    }

    let status = response.status();
    let error_body = response.text().await.unwrap_or_default();
    anyhow::bail!("Mailtrap API returned {status}: {error_body}");
}

async fn send_via_smtp(
    config: &SmtpConfig,
    recipient: &str,
    subject: &str,
    body: &str,
) -> Result<()> {
    let message = Message::builder()
        .from(config.from_address.parse().context("invalid SMTP_FROM address")?)
        .to(recipient.parse().context("invalid recipient email address")?)
        .subject(subject)
        .header(ContentType::TEXT_PLAIN)
        .body(body.to_string())?;

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
