use chrono::Utc;
use tracing::{info, warn};

use crate::{
    crypto::decrypt_secret,
    db::{
        list_capsules_pending_notification, mark_capsule_ready, mark_notification_sent, AppState,
    },
    email::send_ready_notification,
    models::CapsuleStatus,
};

const UNLOCK_POLL_INTERVAL_SECS: u64 = 30;

pub fn spawn_unlock_scheduler(state: AppState) {
    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(UNLOCK_POLL_INTERVAL_SECS));

        loop {
            interval.tick().await;
            if let Err(err) = run_unlock_pass(&state).await {
                tracing::error!(error = %err, "unlock scheduler pass failed");
            }
        }
    });
}

async fn run_unlock_pass(state: &AppState) -> anyhow::Result<()> {
    let now = Utc::now();
    let capsules = list_capsules_pending_notification(state, now).await?;

    for capsule in capsules {
        if let Err(err) = process_capsule(state, &capsule).await {
            warn!(capsule_id = %capsule.id, error = %err, "failed to process capsule notification");
        }
    }

    Ok(())
}

async fn process_capsule(
    state: &AppState,
    capsule: &crate::models::CapsuleRecord,
) -> anyhow::Result<()> {
    if capsule.status == CapsuleStatus::Sealed {
        mark_capsule_ready(state, capsule.id).await?;
        info!(capsule_id = %capsule.id, "capsule marked ready for unlock");
    }

    let Some(email_config) = state.email.as_ref().as_ref() else {
        return Ok(());
    };

    let (encrypted_token, nonce) = match (
        capsule.encrypted_unlock_token.as_deref(),
        capsule.unlock_token_nonce,
    ) {
        (Some(token), Some(nonce)) => (token, nonce),
        _ => return Ok(()),
    };

    let token_bytes = decrypt_secret(state.master_key.as_ref(), encrypted_token, &nonce)?;
    let unlock_token = String::from_utf8(token_bytes)
        .map_err(|_| anyhow::anyhow!("stored unlock token is not valid UTF-8"))?;

    let unlock_url = email_config.unlock_url(&unlock_token);

    match send_ready_notification(email_config, &capsule.recipient_email, &unlock_url).await {
        Ok(()) => {
            mark_notification_sent(state, capsule.id, Utc::now()).await?;
            info!(
                capsule_id = %capsule.id,
                recipient = %capsule.recipient_email,
                "ready notification email sent"
            );
        }
        Err(err) => {
            warn!(
                capsule_id = %capsule.id,
                recipient = %capsule.recipient_email,
                error = %err,
                "failed to send ready notification email; will retry"
            );
        }
    }

    Ok(())
}
