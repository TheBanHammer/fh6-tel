use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use super::ServerState;
use crate::{api, db, parser, settings};

type JsonErr = (StatusCode, Json<serde_json::Value>);

fn err(e: String) -> JsonErr {
    (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e })))
}

pub async fn version() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "version": env!("CARGO_PKG_VERSION") }))
}

pub async fn list_sessions(State(s): State<ServerState>) -> Result<Json<Vec<db::SessionRow>>, JsonErr> {
    api::list_sessions(&s.app).map(Json).map_err(err)
}

pub async fn session_packets(
    State(s): State<ServerState>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<parser::TelemetryPacket>>, JsonErr> {
    api::session_packets(&s.app, id).map(Json).map_err(err)
}

pub async fn session_laps(
    State(s): State<ServerState>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<db::LapRow>>, JsonErr> {
    api::session_laps(&s.app, id).map(Json).map_err(err)
}

pub async fn delete_session(
    State(s): State<ServerState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, JsonErr> {
    api::delete_session(&s.app, id).map(|_| StatusCode::NO_CONTENT).map_err(err)
}

pub async fn clear_all_sessions(State(s): State<ServerState>) -> Result<StatusCode, JsonErr> {
    api::clear_all_sessions(&s.app).map(|_| StatusCode::NO_CONTENT).map_err(err)
}

#[derive(Deserialize)]
pub struct RenameBody { pub name: Option<String> }

pub async fn rename_session(
    State(s): State<ServerState>,
    Path(id): Path<i64>,
    Json(body): Json<RenameBody>,
) -> Result<StatusCode, JsonErr> {
    api::rename_session(&s.app, id, body.name).map(|_| StatusCode::NO_CONTENT).map_err(err)
}

#[derive(Deserialize)]
pub struct BookmarkBody { pub bookmarked: bool }

pub async fn set_bookmark(
    State(s): State<ServerState>,
    Path(id): Path<i64>,
    Json(body): Json<BookmarkBody>,
) -> Result<StatusCode, JsonErr> {
    api::set_session_bookmark(&s.app, id, body.bookmarked).map(|_| StatusCode::NO_CONTENT).map_err(err)
}

pub async fn get_settings(State(s): State<ServerState>) -> Json<settings::Settings> {
    Json(api::get_settings(&s.app))
}

pub async fn save_settings(
    State(s): State<ServerState>,
    Json(new_settings): Json<settings::Settings>,
) -> Result<StatusCode, JsonErr> {
    api::save_settings(&s.app, new_settings).map(|_| StatusCode::NO_CONTENT).map_err(err)
}

#[cfg(test)]
mod tests {
    use crate::server::{auth::AuthState, router, ServerState};
    use crate::{db, session::SessionManager, settings::Settings, AppState};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use std::sync::{Arc, Mutex};
    use tower::ServiceExt;

    fn state() -> ServerState {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init(&conn).unwrap();
        let app = Arc::new(AppState {
            db: Mutex::new(conn),
            session_manager: Mutex::new(SessionManager::new(true)),
            settings: Mutex::new(Settings::default()),
        });
        let (tx, _rx) = tokio::sync::broadcast::channel(16);
        ServerState { app, tx, auth: AuthState::disabled() }
    }

    #[tokio::test]
    async fn sessions_empty_then_listed() {
        let st = state();
        let app = router(st.clone());
        let res = app.clone().oneshot(Request::builder().uri("/api/sessions").body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = axum::body::to_bytes(res.into_body(), 1 << 20).await.unwrap();
        assert_eq!(&body[..], b"[]");
        {
            let conn = st.app.db.lock().unwrap();
            db::open_session(&conn, 1000, 42, 5, 800).unwrap();
        }
        let res = app.oneshot(Request::builder().uri("/api/sessions").body(Body::empty()).unwrap()).await.unwrap();
        let body = axum::body::to_bytes(res.into_body(), 1 << 20).await.unwrap();
        let rows: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[tokio::test]
    async fn version_endpoint() {
        let app = router(state());
        let res = app.oneshot(Request::builder().uri("/api/version").body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn auth_blocks_then_allows() {
        let mut st = state();
        st.auth = AuthState::new(Some("secret".into()));
        let app = router(st.clone());
        // No cookie -> 401 on API.
        let res = app.clone().oneshot(Request::builder().uri("/api/sessions").body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        // Login with correct token -> Set-Cookie.
        let res = app.clone().oneshot(
            Request::builder().method("POST").uri("/login")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("token=secret")).unwrap()
        ).await.unwrap();
        let cookie = res.headers().get("set-cookie").unwrap().to_str().unwrap().to_string();
        let sid = cookie.split(';').next().unwrap().to_string();
        // Reuse cookie -> 200.
        let res = app.oneshot(Request::builder().uri("/api/sessions").header("cookie", sid).body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
}
