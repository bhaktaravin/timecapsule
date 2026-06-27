use chrono::Utc;
use tracing::info;

use crate::db::{list_capsules_ready_to_unlock, mark_capsule_ready, AppState};

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
    let ready_ids = list_capsules_ready_to_unlock(state, now).await?;

    for id in ready_ids {
        mark_capsule_ready(state, id).await?;
        info!(capsule_id = %id, "capsule marked ready for unlock");
    }

    Ok(())
}
