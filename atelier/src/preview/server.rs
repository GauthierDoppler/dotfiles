use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::os::unix::process::CommandExt;
use std::process::Command as Process;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use axum::body::{Body, Bytes};
use axum::extract::{Request, State};
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Router;
use serde_json::{json, Value};
use tokio::sync::broadcast;

use super::{is_markdown, path_from_url};
use crate::Result;

const POLL: Duration = Duration::from_millis(250);
const PING: Duration = Duration::from_secs(30);
const MAX_BODY: usize = 1024 * 1024;

const CSP: &str = "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; \
                   img-src * data: blob:; font-src 'self' data:; connect-src 'self'; \
                   base-uri 'none'; form-action 'none'; frame-ancestors 'none'";

const PAGE: &[u8] = include_bytes!("../../assets/preview/index.html");
const APP: &[u8] = include_bytes!("../../assets/preview/app.js");

const LIBS: &[(&str, &[u8])] = &[
    (
        "markdown-it.js",
        include_bytes!("../../assets/preview/lib/markdown-it.js"),
    ),
    (
        "markdown-it-anchor.js",
        include_bytes!("../../assets/preview/lib/markdown-it-anchor.js"),
    ),
    (
        "markdown-it-footnote.js",
        include_bytes!("../../assets/preview/lib/markdown-it-footnote.js"),
    ),
    (
        "markdown-it-task-lists.js",
        include_bytes!("../../assets/preview/lib/markdown-it-task-lists.js"),
    ),
    (
        "highlight.js",
        include_bytes!("../../assets/preview/lib/highlight.js"),
    ),
    (
        "mermaid.js",
        include_bytes!("../../assets/preview/lib/mermaid.js"),
    ),
    (
        "purify.js",
        include_bytes!("../../assets/preview/lib/purify.js"),
    ),
    (
        "js-yaml.js",
        include_bytes!("../../assets/preview/lib/js-yaml.js"),
    ),
];

struct Watch {
    tx: broadcast::Sender<Bytes>,
    stamp: Option<(SystemTime, u64)>,
    cursor: Option<u64>,
}

struct Server {
    hosts: [String; 2],
    notes: PathBuf,
    watched: Mutex<HashMap<PathBuf, Watch>>,
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
            tokio::spawn(ping(server.clone()));
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
    let boot = stamp(&exe);
    let mut restarting = false;
    let mut tick = tokio::time::interval(POLL);
    loop {
        tick.tick().await;
        {
            let mut watched = server.watched.lock().unwrap();
            watched.retain(|_, watch| watch.tx.receiver_count() > 0 || watch.cursor.is_some());
            for (file, watch) in watched.iter_mut() {
                if watch.tx.receiver_count() == 0 {
                    continue;
                }
                let now = stamp(file);
                if now != watch.stamp {
                    watch.stamp = now;
                    let _ = watch
                        .tx
                        .send(frame("change", &json!({ "gone": now.is_none() })));
                }
            }
        }
        if !restarting {
            if let Some(current) = stamp(&exe) {
                if Some(current) != boot {
                    restarting = restart(&exe);
                }
            }
        }
    }
}

fn restart(exe: &Path) -> bool {
    if cfg!(target_os = "macos") && std::env::var("XPC_SERVICE_NAME").as_deref() == Ok(super::LABEL) {
        return crate::service::restart(super::LABEL);
    }
    let error = Process::new(exe).args(["preview", "serve"]).exec();
    eprintln!("md-preview: cannot restart from {}: {error}", exe.display());
    false
}

async fn ping(server: Arc<Server>) {
    let mut tick = tokio::time::interval(PING);
    loop {
        tick.tick().await;
        for watch in server.watched.lock().unwrap().values() {
            let _ = watch.tx.send(Bytes::from_static(b": ping\n\n"));
        }
    }
}

fn frame(event: &str, data: &Value) -> Bytes {
    Bytes::from(format!("event: {event}\ndata: {data}\n\n"))
}

async fn handle(State(server): State<Arc<Server>>, request: Request) -> Response {
    let mut response = route(&server, request).await;
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(CSP),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}

fn status(code: StatusCode, text: &'static str) -> Response {
    (code, text).into_response()
}

fn typed(content_type: &'static str, body: impl Into<Body>) -> Response {
    ([(header::CONTENT_TYPE, content_type)], body.into()).into_response()
}

fn json_response(value: Value) -> Response {
    typed("application/json", value.to_string())
}

fn no_store(content_type: &'static str, body: impl Into<Body>) -> Response {
    (
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "no-store"),
        ],
        body.into(),
    )
        .into_response()
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
        return no_store("text/javascript; charset=utf-8", APP);
    }
    if let Some(name) = path.strip_prefix("/__lib/") {
        return match LIBS.iter().find(|(lib, _)| *lib == name) {
            Some((_, bytes)) => typed("text/javascript; charset=utf-8", *bytes),
            None => status(StatusCode::NOT_FOUND, "not found"),
        };
    }
    if path.starts_with("/__raw/") {
        return raw(under("/__raw"));
    }
    if path.starts_with("/__events/") {
        return match under("/__events").filter(|file| is_markdown(file) && file.is_file()) {
            Some(file) => events(server, file),
            None => status(StatusCode::NOT_FOUND, "not found"),
        };
    }
    if path.starts_with("/__notes/") {
        let Some(file) = under("/__notes").filter(|file| is_markdown(file)) else {
            return status(StatusCode::BAD_REQUEST, "bad request");
        };
        return match *request.method() {
            Method::GET => get_notes(server, &file),
            Method::PUT => put_notes(server, &file, request).await,
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
        return cursor(server, file, request).await;
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
        return no_store("text/html; charset=utf-8", PAGE);
    }
    asset(server, request.headers(), &file)
}

fn raw(file: Option<PathBuf>) -> Response {
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

fn watch(watched: &mut HashMap<PathBuf, Watch>, file: PathBuf) -> &mut Watch {
    watched.entry(file).or_insert_with_key(|file| Watch {
        tx: broadcast::channel(64).0,
        stamp: stamp(file),
        cursor: None,
    })
}

fn events(server: &Server, file: PathBuf) -> Response {
    let mut watched = server.watched.lock().unwrap();
    let watch = watch(&mut watched, file);
    let mut first = VecDeque::from([Bytes::from_static(b": connected\n\n")]);
    if let Some(line) = watch.cursor {
        first.push_back(frame("cursor", &json!({ "line": line })));
    }
    let rx = watch.tx.subscribe();
    let stream = futures_util::stream::unfold((first, rx), |(mut first, mut rx)| async move {
        if let Some(chunk) = first.pop_front() {
            return Some((Ok::<_, std::convert::Infallible>(chunk), (first, rx)));
        }
        loop {
            match rx.recv().await {
                Ok(chunk) => return Some((Ok(chunk), (first, rx))),
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    });
    no_store("text/event-stream", Body::from_stream(stream))
}

async fn json_body(request: Request) -> Option<Value> {
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

fn notes_file(server: &Server, file: &Path) -> PathBuf {
    let digest = sha1_smol::Sha1::from(file.to_string_lossy().as_bytes()).digest();
    server.notes.join(format!("{digest}.json"))
}

fn get_notes(server: &Server, file: &Path) -> Response {
    let notes = std::fs::read(notes_file(server, file))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|mut stored| stored.get_mut("notes").map(Value::take))
        .filter(Value::is_array)
        .unwrap_or_else(|| json!([]));
    json_response(json!({ "notes": notes }))
}

async fn put_notes(server: &Server, file: &Path, request: Request) -> Response {
    let Some(Value::Array(notes)) = json_body(request)
        .await
        .and_then(|mut body| body.get_mut("notes").map(Value::take))
    else {
        return status(StatusCode::BAD_REQUEST, "bad request");
    };
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

async fn cursor(server: &Server, file: PathBuf, request: Request) -> Response {
    let line = json_body(request)
        .await
        .and_then(|body| body.get("line").and_then(Value::as_u64))
        .filter(|line| *line >= 1);
    let Some(line) = line else {
        return status(StatusCode::BAD_REQUEST, "bad request");
    };
    let mut watched = server.watched.lock().unwrap();
    let watch = watch(&mut watched, file);
    watch.cursor = Some(line);
    let _ = watch.tx.send(frame("cursor", &json!({ "line": line })));
    StatusCode::NO_CONTENT.into_response()
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

fn asset(server: &Server, headers: &HeaderMap, path: &Path) -> Response {
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
