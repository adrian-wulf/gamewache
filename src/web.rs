use axum::body::Body;
use axum::http::{header, Response, StatusCode, Uri};
use axum::response::IntoResponse;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "web/"]
pub struct WebAssets;

pub async fn static_handler(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    let file_path = if path.is_empty()
        || path == "crashes"
        || path.starts_with("crashes/")
        || path == "replays"
        || path == "map"
        || path == "tuning"
        || path == "cache"
        || path == "audit"
        || path == "feedback"
    {
        "index.html"
    } else {
        path
    };

    match WebAssets::get(file_path) {
        Some(content) => {
            let mime = mime_guess::from_path(file_path).first_or_octet_stream();
            Response::builder()
                .header(header::CONTENT_TYPE, mime.as_ref())
                .body(Body::from(content.data))
                .unwrap()
        }
        None => match WebAssets::get("index.html") {
            Some(content) => Response::builder()
                .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                .body(Body::from(content.data))
                .unwrap(),
            None => Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("Dashboard asset not found"))
                .unwrap(),
        },
    }
}
