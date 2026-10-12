use std::path::{Path, PathBuf};

use axum::http::{header, HeaderMap, StatusCode};
use axum::response::Response;

use super::http::{no_store, status, typed};
use super::{is_markdown, path_from_url, Server};

pub(super) fn raw(file: Option<PathBuf>) -> Response {
    let Some(target) = file
        .and_then(|file| file.canonicalize().ok())
        .filter(|target| is_markdown(target) && target.is_file())
    else {
        return status(StatusCode::NOT_FOUND, "gone");
    };
    match std::fs::read(&target) {
        Ok(bytes) => no_store("text/plain; charset=utf-8", bytes),
        Err(_) => status(StatusCode::NOT_FOUND, "gone"),
    }
}

fn referrer_doc(server: &Server, headers: &HeaderMap) -> Option<PathBuf> {
    let referer = headers.get(header::REFERER)?.to_str().ok()?;
    let rest = referer
        .strip_prefix("http://")
        .or_else(|| referer.strip_prefix("https://"))?;
    let (host, path) = rest.split_at(rest.find('/')?);
    if !server.hosts.iter().any(|known| known == host) {
        return None;
    }
    let path = path.split(['?', '#']).next()?;
    let doc = path_from_url(path)?;
    (is_markdown(&doc) && doc.is_file()).then_some(doc)
}

fn root_of(server: &Server, doc: &Path) -> PathBuf {
    let dir = doc.parent().unwrap_or(Path::new("/")).to_path_buf();
    let mut roots = server.roots.lock().unwrap();
    roots
        .entry(dir)
        .or_insert_with_key(|dir| {
            let top = crate::git::toplevel(dir).unwrap_or_else(|| dir.clone());
            top.canonicalize().unwrap_or(top)
        })
        .clone()
}

pub(super) fn asset(server: &Server, headers: &HeaderMap, path: &Path) -> Response {
    let Some(doc) = referrer_doc(server, headers) else {
        return status(StatusCode::FORBIDDEN, "forbidden");
    };
    let Some(target) = path.canonicalize().ok().filter(|target| target.is_file()) else {
        return status(StatusCode::NOT_FOUND, "not found");
    };
    let root = root_of(server, &doc);
    let hidden = target.strip_prefix(&root).map(|inside| {
        inside
            .components()
            .any(|part| part.as_os_str().to_string_lossy().starts_with('.'))
    });
    if hidden != Ok(false) {
        return status(StatusCode::FORBIDDEN, "forbidden");
    }
    match std::fs::read(&target) {
        Ok(bytes) => typed(content_type(&target), bytes),
        Err(_) => status(StatusCode::NOT_FOUND, "not found"),
    }
}

fn content_type(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "ico" => "image/x-icon",
        "bmp" => "image/bmp",
        "pdf" => "application/pdf",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" => "application/json",
        "html" | "htm" => "text/html; charset=utf-8",
        "txt" | "csv" | "log" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}
