use axum::{
    extract::State,
    http::{header, StatusCode},
    middleware::Next,
    response::{Html, IntoResponse, Redirect, Response},
};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use super::ServerState;

const COOKIE: &str = "fh6_session";

#[derive(Clone)]
pub struct AuthState {
    token: Option<Arc<String>>,
    /// Active session ids. In-memory only: a server restart logs everyone out,
    /// and ids are never expired or capped (fine for a LAN tool with infrequent
    /// logins; `logout` removes the caller's id).
    sessions: Arc<Mutex<HashSet<String>>>,
}

impl AuthState {
    pub fn new(token: Option<String>) -> Self {
        Self { token: token.map(Arc::new), sessions: Arc::new(Mutex::new(HashSet::new())) }
    }
    pub fn disabled() -> Self { Self::new(None) }
    pub fn enabled(&self) -> bool { self.token.is_some() }
    fn valid_session(&self, sid: &str) -> bool { self.sessions.lock().unwrap().contains(sid) }
    fn mint(&self) -> String {
        let sid = format!("{:032x}", random_u128());
        self.sessions.lock().unwrap().insert(sid.clone());
        sid
    }
    fn revoke(&self, sid: &str) { self.sessions.lock().unwrap().remove(sid); }
}

// Dependency-free unguessable id. Swap for `rand` if stronger ids are ever needed.
fn random_u128() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let mut x = t.as_nanos() ^ (t.as_secs() as u128).wrapping_mul(0x9E3779B97F4A7C15);
    x ^= x >> 33; x = x.wrapping_mul(0xFF51AFD7ED558CCD);
    x ^= x >> 33; x = x.wrapping_mul(0xC4CEB9FE1A85EC53);
    x ^= x >> 33;
    x
}

fn cookie_value(headers: &header::HeaderMap) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    raw.split(';').filter_map(|kv| {
        let kv = kv.trim();
        let (k, v) = kv.split_once('=')?;
        (k == COOKIE).then(|| v.to_string())
    }).next()
}

fn token_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() { return false; }
    let mut diff = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) { diff |= x ^ y; }
    diff == 0
}

pub async fn guard(State(s): State<ServerState>, req: axum::extract::Request, next: Next) -> Response {
    if !s.auth.enabled() { return next.run(req).await; }
    let path = req.uri().path();
    if path == "/login" || path == "/logout" { return next.run(req).await; }
    let authed = cookie_value(req.headers()).map(|sid| s.auth.valid_session(&sid)).unwrap_or(false);
    if authed { return next.run(req).await; }
    if path.starts_with("/api") || path == "/events" {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }
    login_page().await.into_response()
}

#[derive(serde::Deserialize)]
pub struct LoginForm { pub token: String }

pub async fn login(State(s): State<ServerState>, axum::extract::Form(form): axum::extract::Form<LoginForm>) -> Response {
    let Some(expected) = s.auth.token.as_ref() else { return Redirect::to("/").into_response(); };
    if token_eq(&form.token, expected) {
        let sid = s.auth.mint();
        let cookie = format!("{COOKIE}={sid}; HttpOnly; SameSite=Strict; Path=/");
        ([(header::SET_COOKIE, cookie)], Redirect::to("/")).into_response()
    } else {
        (StatusCode::UNAUTHORIZED, Html(login_html(true))).into_response()
    }
}

pub async fn logout(State(s): State<ServerState>, headers: header::HeaderMap) -> Response {
    if let Some(sid) = cookie_value(&headers) { s.auth.revoke(&sid); }
    let expire = format!("{COOKIE}=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0");
    ([(header::SET_COOKIE, expire)], Redirect::to("/login")).into_response()
}

pub async fn login_page() -> Html<String> { Html(login_html(false)) }

fn login_html(failed: bool) -> String {
    let msg = if failed { "<p style=\"color:#e66\">Incorrect token.</p>" } else { "" };
    format!(
        "<!doctype html><html><head><meta charset=utf-8><title>FH6 Telemetry — Login</title>\
         <meta name=viewport content=\"width=device-width,initial-scale=1\">\
         <style>body{{background:#111;color:#eee;font-family:system-ui;display:grid;place-items:center;height:100vh;margin:0}}\
         form{{background:#1c1c1c;padding:2rem;border-radius:12px;min-width:280px}}\
         input,button{{width:100%;padding:.6rem;margin-top:.5rem;border-radius:8px;border:1px solid #333;background:#222;color:#eee;box-sizing:border-box}}\
         button{{background:#3a7;border:0;cursor:pointer;font-weight:600}}</style></head>\
         <body><form method=post action=/login><h2>FH6 Telemetry</h2>{msg}\
         <input type=password name=token placeholder=\"Access token\" autofocus>\
         <button type=submit>Enter</button></form></body></html>"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn token_eq_works() {
        assert!(token_eq("abc", "abc"));
        assert!(!token_eq("abc", "abd"));
        assert!(!token_eq("abc", "abcd"));
    }
    #[test]
    fn mint_then_validate_then_revoke() {
        let a = AuthState::new(Some("secret".into()));
        let sid = a.mint();
        assert!(a.valid_session(&sid));
        a.revoke(&sid);
        assert!(!a.valid_session(&sid));
    }
}
