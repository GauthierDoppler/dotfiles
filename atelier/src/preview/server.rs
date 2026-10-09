mod assets;
mod events;
mod files;
mod http;
mod notes;
mod restart;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use axum::extract::{Request, State};
use axum::http::{header, HeaderValue, Method, StatusCode};
use axum::response::Response;
use axum::Router;
use serde_json::json;

use super::{is_markdown, path_from_url};
use crate::Result;
use http::{json_response, no_store, status, typed};

const POLL: Duration = Duration::from_millis(250);

struct Server {
    hosts: [String; 2],
    notes: PathBuf,
    watched: Mutex<HashMap<PathBuf, events::Watch>>,
    roots: Mutex<HashMap<PathBuf, PathBuf>>,
}

pub fn serve(port: u16) -> Result<()> {
    let home = crate::dotfiles::home()?;
    let server = Arc::new(Server {
        hosts: [format!("127.0.0.1:{port}"), format!("localhost:{port}")],
        notes: Path::new(&home).join(".local/share/md-preview/notes"),
        watched: Mutex::new(HashMap::new()),
        roots: Mutex::new(HashMap::new()),
    });
    let exe = std::env::current_exe()?;
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
            println!(
                "md-preview: listening on http://127.0.0.1:{port} (pid {})",
                std::process::id()
            );
            tokio::spawn(poll(server.clone(), exe));
            tokio::spawn(events::ping(server.clone()));
            let app = Router::new().fallback(handle).with_state(server);
            axum::serve(listener, app).await?;
            Ok(())
        })
}

fn stamp(path: &Path) -> Option<(SystemTime, u64)> {
    let meta = std::fs::metadata(path).ok()?;
    Some((meta.modified().ok()?, meta.len()))
}

async fn poll(server: Arc<Server>, exe: PathBuf) {
    let mut binary = restart::Binary::new(exe);
    let mut tick = tokio::time::interval(POLL);
    loop {
        tick.tick().await;
        events::announce_changes(&server);
        binary.restart_if_replaced();
    }
}

async fn handle(State(server): State<Arc<Server>>, request: Request) -> Response {
    let mut response = route(&server, request).await;
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(assets::CSP),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}

async fn route(server: &Server, request: Request) -> Response {
    let host = request
        .headers()
        .get(header::HOST)
        .and_then(|host| host.to_str().ok());
    if !host.is_some_and(|host| server.hosts.iter().any(|known| known == host)) {
        return status(StatusCode::FORBIDDEN, "forbidden");
    }

    let path = request.uri().path().to_string();
    let under = |prefix: &str| path.strip_prefix(prefix).and_then(path_from_url);

    if path == "/__meta" {
        return json_response(json!({ "app": "md-preview", "pid": std::process::id() }));
    }
    if path == "/__app.js" {
        return no_store("text/javascript; charset=utf-8", assets::APP);
    }
    if let Some(name) = path.strip_prefix("/__lib/") {
        return match assets::lib(name) {
            Some(bytes) => typed("text/javascript; charset=utf-8", bytes),
            None => status(StatusCode::NOT_FOUND, "not found"),
        };
    }
    if path.starts_with("/__raw/") {
        return files::raw(under("/__raw"));
    }
    if path.starts_with("/__events/") {
        return match under("/__events").filter(|file| is_markdown(file) && file.is_file()) {
            Some(file) => events::events(server, file),
            None => status(StatusCode::NOT_FOUND, "not found"),
        };
    }
    if path.starts_with("/__notes/") {
        let Some(file) = under("/__notes").filter(|file| is_markdown(file)) else {
            return status(StatusCode::BAD_REQUEST, "bad request");
        };
        return match *request.method() {
            Method::GET => notes::get(server, &file),
            Method::PUT => notes::put(server, &file, request).await,
            _ => status(StatusCode::METHOD_NOT_ALLOWED, "method not allowed"),
        };
    }
    if path.starts_with("/__cursor/") {
        if request.method() != Method::POST {
            return status(StatusCode::METHOD_NOT_ALLOWED, "method not allowed");
        }
        let Some(file) = under("/__cursor").filter(|file| is_markdown(file)) else {
            return status(StatusCode::BAD_REQUEST, "bad request");
        };
        return events::cursor(server, file, request).await;
    }
    if path == "/" {
        return typed(
            "text/plain; charset=utf-8",
            "md-preview: open a file with `atelier preview <file.md>`\n",
        );
    }

    let Some(file) = path_from_url(&path) else {
        return status(StatusCode::BAD_REQUEST, "bad request");
    };
    if is_markdown(&file) {
        return no_store("text/html; charset=utf-8", assets::PAGE);
    }
    files::asset(server, request.headers(), &file)
}
