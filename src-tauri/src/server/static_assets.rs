use axum::{
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use rust_embed::RustEmbed;

/// Embeds the SvelteKit static build. Path is relative to this crate (src-tauri),
/// so `../build` is the repo's frontend output.
#[derive(RustEmbed)]
#[folder = "../build"]
struct Assets;

pub async fn serve(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    match Assets::get(path) {
        Some(content) => {
            ([(header::CONTENT_TYPE, mime_for(path))], content.data.into_owned()).into_response()
        }
        // SPA fallback: unknown non-asset routes return index.html.
        None => match Assets::get("index.html") {
            Some(index) => ([(header::CONTENT_TYPE, "text/html")], index.data.into_owned()).into_response(),
            None => (StatusCode::NOT_FOUND, "build assets missing").into_response(),
        },
    }
}

fn mime_for(path: &str) -> &'static str {
    if path.ends_with(".html") { "text/html" }
    else if path.ends_with(".js") { "text/javascript" }
    else if path.ends_with(".css") { "text/css" }
    else if path.ends_with(".json") { "application/json" }
    else if path.ends_with(".svg") { "image/svg+xml" }
    else if path.ends_with(".png") { "image/png" }
    else if path.ends_with(".webp") { "image/webp" }
    else if path.ends_with(".woff2") { "font/woff2" }
    else if path.ends_with(".ico") { "image/x-icon" }
    else { "application/octet-stream" }
}
