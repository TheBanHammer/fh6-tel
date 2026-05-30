pub mod auth;
pub mod routes;
pub mod sse;
pub mod static_assets;

use tokio::sync::broadcast;

use crate::{event::ServerEvent, Shared};

/// Everything the axum handlers need, cloned into each request.
#[derive(Clone)]
pub struct ServerState {
    pub app: Shared,
    pub tx: broadcast::Sender<ServerEvent>,
    pub auth: auth::AuthState,
}

/// Build the full router.
pub fn router(state: ServerState) -> axum::Router {
    use axum::routing::{delete, get, post};

    let api = axum::Router::new()
        .route("/version", get(routes::version))
        .route("/sessions", get(routes::list_sessions))
        .route("/sessions", delete(routes::clear_all_sessions))
        .route("/sessions/:id/packets", get(routes::session_packets))
        .route("/sessions/:id/laps", get(routes::session_laps))
        .route("/sessions/:id", delete(routes::delete_session))
        .route("/sessions/:id/rename", post(routes::rename_session))
        .route("/sessions/:id/bookmark", post(routes::set_bookmark))
        .route("/settings", get(routes::get_settings))
        .route("/settings", post(routes::save_settings));

    axum::Router::new()
        .nest("/api", api)
        .route("/events", get(sse::events))
        .route("/login", get(auth::login_page).post(auth::login))
        .route("/logout", post(auth::logout))
        .fallback(static_assets::serve)
        .layer(axum::middleware::from_fn_with_state(state.clone(), auth::guard))
        .with_state(state)
}
