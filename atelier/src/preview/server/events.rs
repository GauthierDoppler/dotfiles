use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use axum::body::{Body, Bytes};
use axum::extract::Request;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::{json, Value};
use tokio::sync::broadcast;

use super::http::{json_body, no_store, status};
use super::{stamp, Server};

const PING: Duration = Duration::from_secs(30);

pub(super) struct Watch {
    tx: broadcast::Sender<Bytes>,
    stamp: Option<(SystemTime, u64)>,
    cursor: Option<u64>,
}

pub(super) fn announce_changes(server: &Server) {
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

pub(super) async fn ping(server: Arc<Server>) {
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

fn watch(watched: &mut HashMap<PathBuf, Watch>, file: PathBuf) -> &mut Watch {
    watched.entry(file).or_insert_with_key(|file| Watch {
        tx: broadcast::channel(64).0,
        stamp: stamp(file),
        cursor: None,
    })
}

pub(super) fn events(server: &Server, file: PathBuf) -> Response {
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

pub(super) async fn cursor(server: &Server, file: PathBuf, request: Request) -> Response {
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
