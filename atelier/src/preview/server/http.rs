use axum::body::Body;
use axum::extract::Request;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::Value;

const MAX_BODY: usize = 1024 * 1024;

pub(super) fn status(code: StatusCode, text: &'static str) -> Response {
    (code, text).into_response()
}

pub(super) fn typed(content_type: &'static str, body: impl Into<Body>) -> Response {
    ([(header::CONTENT_TYPE, content_type)], body.into()).into_response()
}

pub(super) fn json_response(value: Value) -> Response {
    typed("application/json", value.to_string())
}

pub(super) fn no_store(content_type: &'static str, body: impl Into<Body>) -> Response {
    (
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "no-store"),
        ],
        body.into(),
    )
        .into_response()
}

pub(super) async fn json_body(request: Request) -> Option<Value> {
    let content_type = request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    if content_type.split(';').next().map(str::trim) != Some("application/json") {
        return None;
    }
    let bytes = axum::body::to_bytes(request.into_body(), MAX_BODY)
        .await
        .ok()?;
    serde_json::from_slice(&bytes).ok()
}
