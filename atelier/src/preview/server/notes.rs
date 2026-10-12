use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use axum::extract::Request;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::{json, Value};

use super::http::{json_body, json_response, status};
use super::Server;

fn notes_file(server: &Server, file: &Path) -> PathBuf {
    let digest = sha1_smol::Sha1::from(file.to_string_lossy().as_bytes()).digest();
    server.notes.join(format!("{digest}.json"))
}

pub(super) fn get(server: &Server, file: &Path) -> Response {
    let notes = std::fs::read(notes_file(server, file))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|mut stored| stored.get_mut("notes").map(Value::take))
        .filter(Value::is_array)
        .unwrap_or_else(|| json!([]));
    json_response(json!({ "notes": notes }))
}

pub(super) async fn put(server: Arc<Server>, file: PathBuf, request: Request) -> Response {
    let Some(Value::Array(notes)) = json_body(request)
        .await
        .and_then(|mut body| body.get_mut("notes").map(Value::take))
    else {
        return status(StatusCode::BAD_REQUEST, "bad request");
    };
    super::off_thread(move || save(&server, &file, notes)).await
}

fn save(server: &Server, file: &Path, notes: Vec<Value>) -> Response {
    let dest = notes_file(server, file);
    let written = if notes.is_empty() {
        match std::fs::remove_file(&dest) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error),
            _ => Ok(()),
        }
    } else {
        write_notes(&server.notes, &dest, file, notes)
    };
    match written {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(_) => status(StatusCode::INTERNAL_SERVER_ERROR, "notes not saved"),
    }
}

fn write_notes(dir: &Path, dest: &Path, file: &Path, notes: Vec<Value>) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let mut text = serde_json::to_string_pretty(&json!({
        "path": file.to_string_lossy(),
        "notes": notes,
    }))?;
    text.push('\n');
    static SAVES: AtomicU64 = AtomicU64::new(0);
    let save = SAVES.fetch_add(1, Ordering::Relaxed);
    let tmp = dest.with_extension(format!("json.{}.{save}.tmp", std::process::id()));
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, dest)
}
