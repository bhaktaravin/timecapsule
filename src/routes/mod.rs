mod capsules;

use axum::{
    routing::{get, post},
    Router,
};
use tower_http::services::ServeDir;

use crate::db::AppState;

pub fn router() -> Router<AppState> {
    let api = Router::new()
        .route("/api/capsules", post(capsules::create_capsule))
        .route("/api/capsules/{id}/status", get(capsules::get_capsule_status))
        .route("/api/unlock/{token}", get(capsules::unlock_capsule))
        .route("/api/dev/enabled", get(capsules::dev_enabled))
        .route("/api/dev/test-capsule", post(capsules::create_test_capsule));

    let web = Router::new().fallback_service(ServeDir::new("static").append_index_html_on_directories(true));

    Router::new().merge(api).merge(web)
}
