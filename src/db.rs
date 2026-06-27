use std::sync::Arc;

use chrono::{DateTime, Utc};
use sqlx::{sqlite::SqlitePoolOptions, FromRow, SqlitePool};
use uuid::Uuid;

use crate::{
    config::Config,
    email::SharedEmailConfig,
    error::{AppError, AppResult},
    models::{CapsuleRecord, CapsuleStatus},
};

const CAPSULE_SELECT: &str = r#"
    SELECT
        id,
        recipient_email,
        unlock_at,
        encrypted_payload,
        payload_nonce,
        wrapped_dek,
        wrapped_dek_nonce,
        unlock_token_hash,
        encrypted_unlock_token,
        unlock_token_nonce,
        status,
        created_at,
        opened_at,
        notification_sent_at
    FROM capsules
"#;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub master_key: Arc<[u8; 32]>,
    pub dev_mode: bool,
    pub email: SharedEmailConfig,
}

impl AppState {
    pub async fn new(config: &Config, email: SharedEmailConfig) -> anyhow::Result<Self> {
        let db = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&config.database_url)
            .await?;

        sqlx::migrate!("./migrations").run(&db).await?;

        Ok(Self {
            db,
            master_key: Arc::new(config.master_key),
            dev_mode: config.dev_mode,
            email,
        })
    }
}

#[derive(FromRow)]
struct CapsuleRow {
    id: String,
    recipient_email: String,
    unlock_at: String,
    encrypted_payload: Vec<u8>,
    payload_nonce: Vec<u8>,
    wrapped_dek: Vec<u8>,
    wrapped_dek_nonce: Vec<u8>,
    unlock_token_hash: Vec<u8>,
    encrypted_unlock_token: Option<Vec<u8>>,
    unlock_token_nonce: Option<Vec<u8>>,
    status: String,
    created_at: String,
    opened_at: Option<String>,
    notification_sent_at: Option<String>,
}

impl TryFrom<CapsuleRow> for CapsuleRecord {
    type Error = AppError;

    fn try_from(row: CapsuleRow) -> Result<Self, Self::Error> {
        let id = Uuid::parse_str(&row.id)
            .map_err(|_| AppError::Internal(anyhow::anyhow!("invalid capsule id in database")))?;

        Ok(Self {
            id,
            recipient_email: row.recipient_email,
            unlock_at: parse_timestamp(&row.unlock_at, "unlock_at")?,
            encrypted_payload: row.encrypted_payload,
            payload_nonce: to_nonce_array(&row.payload_nonce)?,
            wrapped_dek: row.wrapped_dek,
            wrapped_dek_nonce: to_nonce_array(&row.wrapped_dek_nonce)?,
            unlock_token_hash: to_hash_array(&row.unlock_token_hash)?,
            encrypted_unlock_token: row.encrypted_unlock_token,
            unlock_token_nonce: row
                .unlock_token_nonce
                .map(|value| to_nonce_array(&value))
                .transpose()?,
            status: CapsuleStatus::parse(&row.status).ok_or_else(|| {
                AppError::Internal(anyhow::anyhow!("invalid capsule status in database"))
            })?,
            created_at: parse_timestamp(&row.created_at, "created_at")?,
            opened_at: row
                .opened_at
                .map(|value| parse_timestamp(&value, "opened_at"))
                .transpose()?,
            notification_sent_at: row
                .notification_sent_at
                .map(|value| parse_timestamp(&value, "notification_sent_at"))
                .transpose()?,
        })
    }
}

pub async fn insert_capsule(
    state: &AppState,
    id: Uuid,
    recipient_email: &str,
    unlock_at: DateTime<Utc>,
    encrypted_payload: &[u8],
    payload_nonce: &[u8; 12],
    wrapped_dek: &[u8],
    wrapped_dek_nonce: &[u8; 12],
    unlock_token_hash: &[u8; 32],
    encrypted_unlock_token: &[u8],
    unlock_token_nonce: &[u8; 12],
    created_at: DateTime<Utc>,
) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO capsules (
            id,
            recipient_email,
            unlock_at,
            encrypted_payload,
            payload_nonce,
            wrapped_dek,
            wrapped_dek_nonce,
            unlock_token_hash,
            encrypted_unlock_token,
            unlock_token_nonce,
            status,
            created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(id.to_string())
    .bind(recipient_email)
    .bind(unlock_at.to_rfc3339())
    .bind(encrypted_payload)
    .bind(payload_nonce.as_slice())
    .bind(wrapped_dek)
    .bind(wrapped_dek_nonce.as_slice())
    .bind(unlock_token_hash.as_slice())
    .bind(encrypted_unlock_token)
    .bind(unlock_token_nonce.as_slice())
    .bind(CapsuleStatus::Sealed.as_str())
    .bind(created_at.to_rfc3339())
    .execute(&state.db)
    .await
    .map_err(|err| AppError::Internal(err.into()))?;

    Ok(())
}

pub async fn get_capsule_by_id(state: &AppState, id: Uuid) -> AppResult<CapsuleRecord> {
    let row = sqlx::query_as::<_, CapsuleRow>(&format!("{CAPSULE_SELECT} WHERE id = ?"))
        .bind(id.to_string())
        .fetch_optional(&state.db)
        .await
        .map_err(|err| AppError::Internal(err.into()))?
        .ok_or_else(|| AppError::NotFound("capsule not found".to_string()))?;

    row.try_into()
}

pub async fn get_capsule_by_token_hash(
    state: &AppState,
    token_hash: &[u8; 32],
) -> AppResult<CapsuleRecord> {
    let row = sqlx::query_as::<_, CapsuleRow>(&format!(
        "{CAPSULE_SELECT} WHERE unlock_token_hash = ?"
    ))
    .bind(token_hash.as_slice())
    .fetch_optional(&state.db)
    .await
    .map_err(|err| AppError::Internal(err.into()))?
    .ok_or_else(|| AppError::NotFound("capsule not found".to_string()))?;

    row.try_into()
}

pub async fn mark_capsule_ready(state: &AppState, id: Uuid) -> AppResult<()> {
    sqlx::query(
        r#"
        UPDATE capsules
        SET status = ?
        WHERE id = ? AND status = ?
        "#,
    )
    .bind(CapsuleStatus::Ready.as_str())
    .bind(id.to_string())
    .bind(CapsuleStatus::Sealed.as_str())
    .execute(&state.db)
    .await
    .map_err(|err| AppError::Internal(err.into()))?;

    Ok(())
}

pub async fn mark_capsule_opened(state: &AppState, id: Uuid, opened_at: DateTime<Utc>) -> AppResult<()> {
    sqlx::query(
        r#"
        UPDATE capsules
        SET status = ?, opened_at = ?
        WHERE id = ? AND status != ?
        "#,
    )
    .bind(CapsuleStatus::Opened.as_str())
    .bind(opened_at.to_rfc3339())
    .bind(id.to_string())
    .bind(CapsuleStatus::Opened.as_str())
    .execute(&state.db)
    .await
    .map_err(|err| AppError::Internal(err.into()))?;

    Ok(())
}

pub async fn mark_notification_sent(state: &AppState, id: Uuid, sent_at: DateTime<Utc>) -> AppResult<()> {
    sqlx::query(
        r#"
        UPDATE capsules
        SET notification_sent_at = ?
        WHERE id = ?
        "#,
    )
    .bind(sent_at.to_rfc3339())
    .bind(id.to_string())
    .execute(&state.db)
    .await
    .map_err(|err| AppError::Internal(err.into()))?;

    Ok(())
}

pub async fn list_capsules_pending_notification(
    state: &AppState,
    now: DateTime<Utc>,
) -> AppResult<Vec<CapsuleRecord>> {
    let rows = sqlx::query_as::<_, CapsuleRow>(&format!(
        "{CAPSULE_SELECT}
         WHERE unlock_at <= ?
           AND notification_sent_at IS NULL
           AND status != ?
           AND encrypted_unlock_token IS NOT NULL"
    ))
    .bind(now.to_rfc3339())
    .bind(CapsuleStatus::Opened.as_str())
    .fetch_all(&state.db)
    .await
    .map_err(|err| AppError::Internal(err.into()))?;

    rows.into_iter().map(TryInto::try_into).collect()
}

fn parse_timestamp(value: &str, field: &str) -> AppResult<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| AppError::Internal(anyhow::anyhow!("invalid {field} timestamp in database")))
}

fn to_nonce_array(bytes: &[u8]) -> AppResult<[u8; 12]> {
    if bytes.len() != 12 {
        return Err(AppError::Internal(anyhow::anyhow!(
            "invalid nonce length in database"
        )));
    }
    let mut array = [0u8; 12];
    array.copy_from_slice(bytes);
    Ok(array)
}

fn to_hash_array(bytes: &[u8]) -> AppResult<[u8; 32]> {
    if bytes.len() != 32 {
        return Err(AppError::Internal(anyhow::anyhow!(
            "invalid token hash length in database"
        )));
    }
    let mut array = [0u8; 32];
    array.copy_from_slice(bytes);
    Ok(array)
}

pub fn effective_status(record: &CapsuleRecord, now: DateTime<Utc>) -> CapsuleStatus {
    match record.status {
        CapsuleStatus::Opened => CapsuleStatus::Opened,
        CapsuleStatus::Ready => CapsuleStatus::Ready,
        CapsuleStatus::Sealed if now >= record.unlock_at => CapsuleStatus::Ready,
        CapsuleStatus::Sealed => CapsuleStatus::Sealed,
    }
}

pub fn can_unlock(record: &CapsuleRecord, now: DateTime<Utc>) -> bool {
    matches!(effective_status(record, now), CapsuleStatus::Ready)
}
