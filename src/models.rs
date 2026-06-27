use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapsuleStatus {
    Sealed,
    Ready,
    Opened,
}

impl CapsuleStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sealed => "sealed",
            Self::Ready => "ready",
            Self::Opened => "opened",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "sealed" => Some(Self::Sealed),
            "ready" => Some(Self::Ready),
            "opened" => Some(Self::Opened),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CapsuleRecord {
    pub id: Uuid,
    pub recipient_email: String,
    pub unlock_at: DateTime<Utc>,
    pub encrypted_payload: Vec<u8>,
    pub payload_nonce: [u8; 12],
    pub wrapped_dek: Vec<u8>,
    pub wrapped_dek_nonce: [u8; 12],
    pub unlock_token_hash: [u8; 32],
    pub status: CapsuleStatus,
    pub created_at: DateTime<Utc>,
    pub opened_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCapsuleRequest {
    pub message: String,
    pub recipient_email: String,
    pub unlock_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct CreateCapsuleResponse {
    pub id: Uuid,
    pub recipient_email: String,
    pub unlock_at: DateTime<Utc>,
    pub status: CapsuleStatus,
    pub unlock_token: String,
    pub unlock_path: String,
}

#[derive(Debug, Serialize)]
pub struct CapsuleStatusResponse {
    pub id: Uuid,
    pub recipient_email: String,
    pub unlock_at: DateTime<Utc>,
    pub status: CapsuleStatus,
    pub can_unlock: bool,
    pub opened_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct DevEnabledResponse {
    pub enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct UnlockResponse {
    pub id: Uuid,
    pub recipient_email: String,
    pub unlock_at: DateTime<Utc>,
    pub opened_at: DateTime<Utc>,
    pub message: String,
}
