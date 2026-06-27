use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::{
    crypto::{decrypt_payload, encrypt_payload, encrypt_secret, generate_unlock_token, hash_unlock_token, tokens_equal},
    db::{
        can_unlock, effective_status, get_capsule_by_id, get_capsule_by_token_hash,
        insert_capsule, mark_capsule_opened, AppState,
    },
    error::{AppError, AppResult},
    models::{
        CapsuleStatus, CreateCapsuleRequest, CreateCapsuleResponse, CapsuleStatusResponse,
        DevEnabledResponse, UnlockResponse,
    },
};

const TEST_UNLOCK_SECONDS: i64 = 15;
const TEST_MESSAGE: &str =
    "This is a test message from Timecapsule. If you can read this, unlock works!";
const TEST_RECIPIENT: &str = "test@example.com";

pub async fn create_capsule(
    State(state): State<AppState>,
    Json(request): Json<CreateCapsuleRequest>,
) -> AppResult<Json<CreateCapsuleResponse>> {
    validate_create_request(&request)?;
    create_capsule_record(&state, request).await
}

pub async fn create_test_capsule(
    State(state): State<AppState>,
) -> AppResult<Json<CreateCapsuleResponse>> {
    if !state.dev_mode {
        return Err(AppError::Forbidden(
            "test capsules are disabled; set DEV_MODE=1 to enable".to_string(),
        ));
    }

    let request = CreateCapsuleRequest {
        message: TEST_MESSAGE.to_string(),
        recipient_email: TEST_RECIPIENT.to_string(),
        unlock_at: Utc::now() + Duration::seconds(TEST_UNLOCK_SECONDS),
    };

    create_capsule_record(&state, request).await
}

pub async fn dev_enabled(State(state): State<AppState>) -> Json<DevEnabledResponse> {
    Json(DevEnabledResponse {
        enabled: state.dev_mode,
    })
}

async fn create_capsule_record(
    state: &AppState,
    request: CreateCapsuleRequest,
) -> AppResult<Json<CreateCapsuleResponse>> {
    let encrypted = encrypt_payload(state.master_key.as_ref(), request.message.as_bytes())
        .map_err(AppError::Internal)?;

    let unlock_token = generate_unlock_token();
    let unlock_token_hash = hash_unlock_token(&unlock_token);
    let (encrypted_unlock_token, unlock_token_nonce) =
        encrypt_secret(state.master_key.as_ref(), unlock_token.as_bytes())
            .map_err(AppError::Internal)?;

    let id = Uuid::new_v4();
    let created_at = Utc::now();

    insert_capsule(
        state,
        id,
        &request.recipient_email,
        request.unlock_at,
        &encrypted.ciphertext,
        &encrypted.payload_nonce,
        &encrypted.wrapped_dek,
        &encrypted.wrapped_dek_nonce,
        &unlock_token_hash,
        &encrypted_unlock_token,
        &unlock_token_nonce,
        created_at,
    )
    .await?;

    Ok(Json(CreateCapsuleResponse {
        id,
        recipient_email: request.recipient_email,
        unlock_at: request.unlock_at,
        status: CapsuleStatus::Sealed,
        unlock_path: format!("/api/unlock/{unlock_token}"),
        unlock_token,
    }))
}

pub async fn get_capsule_status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<CapsuleStatusResponse>> {
    let record = get_capsule_by_id(&state, id).await?;
    let now = Utc::now();
    let status = effective_status(&record, now);
    let can_unlock_now = can_unlock(&record, now);

    Ok(Json(CapsuleStatusResponse {
        id: record.id,
        recipient_email: record.recipient_email,
        unlock_at: record.unlock_at,
        can_unlock: can_unlock_now,
        status,
        opened_at: record.opened_at,
    }))
}

pub async fn unlock_capsule(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> AppResult<Json<UnlockResponse>> {
    if token.trim().is_empty() {
        return Err(AppError::BadRequest("unlock token is required".to_string()));
    }

    let token_hash = hash_unlock_token(&token);
    let record = get_capsule_by_token_hash(&state, &token_hash).await?;

    if !tokens_equal(&record.unlock_token_hash, &token_hash) {
        return Err(AppError::NotFound("capsule not found".to_string()));
    }

    let now = Utc::now();
    if now < record.unlock_at {
        return Err(AppError::Forbidden(format!(
            "capsule is sealed until {}",
            record.unlock_at.to_rfc3339()
        )));
    }

    if record.status == CapsuleStatus::Opened {
        return Err(AppError::Forbidden("capsule has already been opened".to_string()));
    }

    let plaintext = decrypt_payload(
        state.master_key.as_ref(),
        &record.encrypted_payload,
        &record.payload_nonce,
        &record.wrapped_dek,
        &record.wrapped_dek_nonce,
    )
    .map_err(AppError::Internal)?;

    let message = String::from_utf8(plaintext).map_err(|_| {
        AppError::Internal(anyhow::anyhow!("stored capsule message is not valid UTF-8"))
    })?;

    mark_capsule_opened(&state, record.id, now).await?;

    Ok(Json(UnlockResponse {
        id: record.id,
        recipient_email: record.recipient_email,
        unlock_at: record.unlock_at,
        opened_at: now,
        message,
    }))
}

fn validate_create_request(request: &CreateCapsuleRequest) -> AppResult<()> {
    let message = request.message.trim();
    if message.is_empty() {
        return Err(AppError::BadRequest("message must not be empty".to_string()));
    }
    if message.len() > 16_384 {
        return Err(AppError::BadRequest(
            "message must be at most 16 KiB".to_string(),
        ));
    }

    let email = request.recipient_email.trim();
    if email.is_empty() || !email.contains('@') {
        return Err(AppError::BadRequest(
            "recipient_email must be a valid email address".to_string(),
        ));
    }

    if request.unlock_at <= Utc::now() {
        return Err(AppError::BadRequest(
            "unlock_at must be in the future".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use chrono::Duration;

    async fn test_state() -> AppState {
        let config = Config {
            master_key: [9u8; 32],
            database_url: "sqlite::memory:".to_string(),
            host: "127.0.0.1".to_string(),
            port: 0,
            dev_mode: true,
        };
        AppState::new(&config, std::sync::Arc::new(None))
            .await
            .expect("test state")
    }

    #[tokio::test]
    async fn test_message_unlock_flow() {
        let state = test_state().await;

        let request = CreateCapsuleRequest {
            message: TEST_MESSAGE.to_string(),
            recipient_email: TEST_RECIPIENT.to_string(),
            unlock_at: Utc::now() + Duration::seconds(1),
        };

        let created = create_capsule_record(&state, request)
            .await
            .expect("create capsule")
            .0;

        let sealed = unlock_capsule(
            State(state.clone()),
            Path(created.unlock_token.clone()),
        )
        .await;
        assert!(sealed.is_err());

        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        let unlocked = unlock_capsule(State(state.clone()), Path(created.unlock_token.clone()))
            .await
            .expect("unlock capsule")
            .0;

        assert_eq!(unlocked.message, TEST_MESSAGE);
        assert_eq!(unlocked.recipient_email, TEST_RECIPIENT);

        let again = unlock_capsule(State(state), Path(created.unlock_token)).await;
        assert!(again.is_err());
    }
}
