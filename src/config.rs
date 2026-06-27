use anyhow::{bail, Context, Result};

#[derive(Clone)]
pub struct Config {
    pub master_key: [u8; 32],
    pub database_url: String,
    pub host: String,
    pub port: u16,
    pub dev_mode: bool,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let master_key_hex = std::env::var("MASTER_KEY").context("MASTER_KEY must be set")?;
        let master_key = parse_master_key(&master_key_hex)?;

        Ok(Self {
            master_key,
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:timecapsule.db?mode=rwc".to_string()),
            host: std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .context("PORT must be a valid u16")?,
            dev_mode: parse_dev_mode(),
        })
    }
}

fn parse_master_key(hex_str: &str) -> Result<[u8; 32]> {
    let bytes = hex::decode(hex_str.trim()).context("MASTER_KEY must be valid hex")?;
    if bytes.len() != 32 {
        bail!("MASTER_KEY must be exactly 32 bytes (64 hex characters)");
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    Ok(key)
}

fn parse_dev_mode() -> bool {
    std::env::var("DEV_MODE")
        .map(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}
