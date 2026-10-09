#![allow(clippy::disallowed_methods)]

mod common;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::Duration;

use common::BoundedOutput;

struct Preview {
    port: u16,
    home: tempfile::TempDir,
    child: Child,
}

struct Response {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Response {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    fn text(&self) -> String {
        String::from_utf8(self.body.clone()).expect("utf-8 body")
    }
}

fn alive(port: u16) -> bool {
    TcpStream::connect(("127.0.0.1", port)).is_ok()
}

impl Preview {
    fn start() -> Self {
        Self::start_with(Path::new(env!("CARGO_BIN_EXE_atelier")))
    }

    fn start_with(binary: &Path) -> Self {
        Self::start_with_path(binary, &std::env::var_os("PATH").unwrap_or_default())
    }

    fn start_with_path(binary: &Path, path: &std::ffi::OsStr) -> Self {
        let port = common::free_port();
        let home = tempfile::tempdir().unwrap();
        let spawn = || {
            Command::new(binary)
                .args(["preview", "serve"])
                .env("HOME", home.path())
                .env("PATH", path)
                .env("MD_PREVIEW_PORT", port.to_string())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
        };
        // A binary copied by this process can stay open for writing in a
        // child forked concurrently by another test, until that child execs.
        let mut attempts = 0;
        let child = loop {
            match spawn() {
                Err(e) if e.kind() == std::io::ErrorKind::ExecutableFileBusy && attempts < 50 => {
                    attempts += 1;
                    std::thread::sleep(Duration::from_millis(20));
                }
                result => break result.expect("server starts"),
            }
        };
        common::wait_until("the preview server", || alive(port));
        Preview { port, home, child }
    }

    fn host(&self) -> String {
        format!("127.0.0.1:{}", self.port)
    }

    fn url(&self, path: &Path) -> String {
        format!("http://{}{}", self.host(), encode(path))
    }

    fn get(&self, path: &str) -> Response {
        self.request("GET", path, &[], None)
    }

    fn request(
        &self,
        method: &str,
        path: &str,
        headers: &[(&str, &str)],
        body: Option<&str>,
    ) -> Response {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let mut head = format!("{method} {path} HTTP/1.1\r\nConnection: close\r\n");
        if !headers
            .iter()
            .any(|(key, _)| key.eq_ignore_ascii_case("host"))
        {
            head.push_str(&format!("Host: {}\r\n", self.host()));
        }
        for (key, value) in headers {
            head.push_str(&format!("{key}: {value}\r\n"));
        }
        let body = body.unwrap_or("");
        head.push_str(&format!("Content-Length: {}\r\n\r\n{body}", body.len()));
        stream.write_all(head.as_bytes()).unwrap();
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).unwrap();
        parse(&raw)
    }

    fn events(&self, file: &Path) -> Events {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        write!(
            stream,
            "GET /__events{} HTTP/1.1\r\nHost: {}\r\n\r\n",
            encode(file),
            self.host()
        )
        .unwrap();
        let mut reader = BufReader::new(stream);
        let mut status = String::new();
        reader.read_line(&mut status).unwrap();
        assert!(status.contains(" 200 "), "events: {status}");
        let mut line = String::new();
        loop {
            line.clear();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" {
                break;
            }
        }
        Events { reader }
    }

    fn notes_dir(&self) -> PathBuf {
        self.home.path().join(".local/share/md-preview/notes")
    }
}

impl Drop for Preview {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

struct Events {
    reader: BufReader<TcpStream>,
}

impl Events {
    fn next_event(&mut self, name: &str) -> String {
        let wanted = format!("event: {name}");
        let mut line = String::new();
        loop {
            line.clear();
            let read = self.reader.read_line(&mut line).expect("event arrives");
            assert!(read > 0, "stream closed before {name}");
            if line.trim_end() == wanted {
                line.clear();
                self.reader.read_line(&mut line).unwrap();
                return line
                    .trim_end()
                    .strip_prefix("data: ")
                    .expect("data line")
                    .to_string();
            }
        }
    }
}

fn parse(raw: &[u8]) -> Response {
    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("complete head");
    let head = String::from_utf8(raw[..split].to_vec()).unwrap();
    let mut lines = head.split("\r\n");
    let status = lines
        .next()
        .unwrap()
        .split(' ')
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    let headers: Vec<(String, String)> = lines
        .map(|line| {
            let (key, value) = line.split_once(':').unwrap();
            (key.to_string(), value.trim().to_string())
        })
        .collect();
    let mut body = raw[split + 4..].to_vec();
    let chunked = headers.iter().any(|(key, value)| {
        key.eq_ignore_ascii_case("transfer-encoding") && value.contains("chunked")
    });
    if chunked {
        body = dechunk(&body);
    }
    Response {
        status,
        headers,
        body,
    }
}

fn dechunk(mut raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let end = raw.windows(2).position(|w| w == b"\r\n").unwrap();
        let size = usize::from_str_radix(std::str::from_utf8(&raw[..end]).unwrap(), 16).unwrap();
        if size == 0 {
            return out;
        }
        out.extend_from_slice(&raw[end + 2..end + 2 + size]);
        raw = &raw[end + 4 + size..];
    }
}

fn encode(path: &Path) -> String {
    path.to_str()
        .unwrap()
        .split('/')
        .map(|segment| {
            segment
                .bytes()
                .map(|b| match b {
                    b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                        (b as char).to_string()
                    }
                    _ => format!("%{b:02X}"),
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn write(path: &Path, content: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
}

#[test]
fn a_markdown_path_serves_the_page_under_a_script_src_self_csp() {
    let server = Preview::start();

    let page = server.get("/anywhere/notes.md");

    assert_eq!(page.status, 200);
    assert!(page
        .header("content-type")
        .unwrap()
        .starts_with("text/html"));
    let csp = page.header("content-security-policy").unwrap();
    assert!(csp.contains("script-src 'self'"), "{csp}");
    assert!(csp.contains("default-src 'none'"), "{csp}");
    assert!(page.text().contains(r#"<script src="/__app.js"></script>"#));
}

#[test]
fn every_script_the_page_loads_is_served_from_the_binary() {
    let server = Preview::start();
    let page = server.get("/doc.md").text();
    let scripts: Vec<&str> = page
        .split(r#"<script src=""#)
        .skip(1)
        .map(|rest| &rest[..rest.find('"').unwrap()])
        .collect();
    assert_eq!(scripts.len(), 9, "{scripts:?}");

    for script in scripts {
        let response = server.get(script);
        assert_eq!(response.status, 200, "{script}");
        assert!(
            response
                .header("content-type")
                .unwrap()
                .starts_with("text/javascript"),
            "{script}"
        );
        assert!(response.body.len() > 1000, "{script} is suspiciously small");
    }
    assert!(server
        .get("/__lib/mermaid.js")
        .text()
        .contains("securityLevel"));
}

#[test]
fn the_page_sanitises_rendered_markdown_and_runs_mermaid_strict() {
    let server = Preview::start();

    let app = server.get("/__app.js").text();

    assert!(app.contains("DOMPurify.sanitize(md.render("));
    assert!(app.contains("securityLevel: 'strict'"));
}

#[test]
fn an_unknown_library_is_not_found() {
    let server = Preview::start();

    assert_eq!(server.get("/__lib/evil.js").status, 404);
    assert_eq!(server.get("/__lib/../app.js").status, 404);
}

#[test]
fn a_foreign_host_header_is_refused_against_dns_rebinding() {
    let server = Preview::start();
    let port = server.port.to_string();

    let rebound = server.request(
        "GET",
        "/__meta",
        &[("Host", &format!("evil.example:{port}"))],
        None,
    );
    let localhost = server.request(
        "GET",
        "/__meta",
        &[("Host", &format!("localhost:{port}"))],
        None,
    );
    let wrong_port = server.request("GET", "/__meta", &[("Host", "127.0.0.1:1")], None);

    assert_eq!(rebound.status, 403);
    assert_eq!(wrong_port.status, 403);
    assert_eq!(localhost.status, 200);
    assert!(localhost.text().contains(r#""app":"md-preview""#));
}

#[test]
fn raw_serves_markdown_source_and_nothing_else() {
    let (_dir, root) = common::real_tempdir();
    write(&root.join("plan.md"), "# Plan\n");
    write(&root.join("secret.txt"), "hunter2");
    let server = Preview::start();

    let markdown = server.get(&format!("/__raw{}", encode(&root.join("plan.md"))));
    let other = server.get(&format!("/__raw{}", encode(&root.join("secret.txt"))));
    let missing = server.get(&format!("/__raw{}", encode(&root.join("gone.md"))));

    assert_eq!(markdown.status, 200);
    assert_eq!(markdown.text(), "# Plan\n");
    assert_eq!(markdown.header("cache-control"), Some("no-store"));
    assert_eq!(other.status, 404);
    assert_eq!(missing.status, 404);
}

#[test]
fn a_path_with_spaces_and_unicode_round_trips_through_the_url() {
    let (_dir, root) = common::real_tempdir();
    let file = root.join("my notes/été #1.md");
    write(&file, "accents");
    let server = Preview::start();

    let raw = server.get(&format!("/__raw{}", encode(&file)));

    assert_eq!(raw.text(), "accents");
}

#[test]
fn a_non_markdown_file_needs_a_markdown_referer() {
    let (_dir, root) = common::real_tempdir();
    let repo = root.join("repo");
    common::git_repo(&repo);
    write(&repo.join("docs/plan.md"), "![](../img/a.png)");
    write(&repo.join("img/a.png"), "PNGDATA");
    let server = Preview::start();
    let image = encode(&repo.join("img/a.png"));

    let bare = server.get(&image);
    let from_page = server.request(
        "GET",
        &image,
        &[("Referer", &server.url(&repo.join("docs/plan.md")))],
        None,
    );
    let from_text = server.request(
        "GET",
        &image,
        &[("Referer", &server.url(&repo.join("img/a.png")))],
        None,
    );

    assert_eq!(bare.status, 403);
    assert_eq!(from_text.status, 403);
    assert_eq!(from_page.status, 200);
    assert_eq!(from_page.body, b"PNGDATA");
    assert_eq!(from_page.header("content-type"), Some("image/png"));
}

#[test]
fn a_markdown_page_cannot_reach_outside_its_repo() {
    let (_dir, root) = common::real_tempdir();
    let repo = root.join("repo");
    common::git_repo(&repo);
    write(&repo.join("README.md"), "hi");
    write(&root.join("elsewhere/id_rsa"), "PRIVATE");
    std::os::unix::fs::symlink(root.join("elsewhere/id_rsa"), repo.join("link")).unwrap();
    let server = Preview::start();
    let referer = server.url(&repo.join("README.md"));

    let outside = server.request(
        "GET",
        &encode(&root.join("elsewhere/id_rsa")),
        &[("Referer", &referer)],
        None,
    );
    let through_symlink = server.request(
        "GET",
        &encode(&repo.join("link")),
        &[("Referer", &referer)],
        None,
    );
    let dot_dot = server.request(
        "GET",
        &format!("{}/../elsewhere/id_rsa", encode(&repo)),
        &[("Referer", &referer)],
        None,
    );

    assert_eq!(outside.status, 403);
    assert_eq!(through_symlink.status, 403);
    assert_eq!(dot_dot.status, 403);
}

#[test]
fn outside_git_a_markdown_page_reaches_only_its_own_directory() {
    let (_dir, root) = common::real_tempdir();
    write(&root.join("notes/today.md"), "hi");
    write(&root.join("notes/sketch.svg"), "<svg/>");
    write(&root.join("other/sketch.svg"), "<svg/>");
    let server = Preview::start();
    let referer = server.url(&root.join("notes/today.md"));

    let sibling = server.request(
        "GET",
        &encode(&root.join("notes/sketch.svg")),
        &[("Referer", &referer)],
        None,
    );
    let elsewhere = server.request(
        "GET",
        &encode(&root.join("other/sketch.svg")),
        &[("Referer", &referer)],
        None,
    );

    assert_eq!(sibling.status, 200);
    assert_eq!(elsewhere.status, 403);
}

#[test]
fn a_referer_from_another_origin_is_not_trusted() {
    let (_dir, root) = common::real_tempdir();
    write(&root.join("doc.md"), "hi");
    write(&root.join("data.json"), "{}");
    let server = Preview::start();

    let response = server.request(
        "GET",
        &encode(&root.join("data.json")),
        &[(
            "Referer",
            &format!("http://evil.example{}", encode(&root.join("doc.md"))),
        )],
        None,
    );

    assert_eq!(response.status, 403);
}

#[test]
fn an_html_file_served_to_a_page_cannot_run_inline_script() {
    let (_dir, root) = common::real_tempdir();
    write(&root.join("doc.md"), "hi");
    write(&root.join("page.html"), "<script>alert(1)</script>");
    let server = Preview::start();

    let response = server.request(
        "GET",
        &encode(&root.join("page.html")),
        &[("Referer", &server.url(&root.join("doc.md")))],
        None,
    );

    assert_eq!(response.status, 200);
    let csp = response.header("content-security-policy").unwrap();
    assert!(csp.contains("script-src 'self'"), "{csp}");
    assert_eq!(response.header("x-content-type-options"), Some("nosniff"));
}

fn sha1_hex(text: &str) -> String {
    sha1_smol::Sha1::from(text).digest().to_string()
}

fn notes_file(doc: &Path) -> String {
    include_str!("fixtures/preview/notes-file.json").replace("{{PATH}}", doc.to_str().unwrap())
}

const NOTES_AS_SENT: &str = r###"{"notes":[{"id":"1759912345678-k3x9a","l0":3,"l1":3,"snip0":"## Rollout","snip1":"## Rollout","text":"say \"when\", not \"soon\"","orphan":false},{"id":"1759912399001-p0q2z","l0":12,"l1":14,"snip0":"- migrate the é/ü table","snip1":"```","text":"two lines?\nsplit — or ✓ merge\tthem","orphan":true}]}"###;

#[test]
fn a_notes_file_on_disk_is_read_back_unchanged() {
    let (_dir, root) = common::real_tempdir();
    let doc = root.join("my plans/é plan.md");
    write(&doc, "# Plan\n");
    let server = Preview::start();
    write(
        &server
            .notes_dir()
            .join(format!("{}.json", sha1_hex(doc.to_str().unwrap()))),
        &notes_file(&doc),
    );

    let notes = server.get(&format!("/__notes{}", encode(&doc)));

    assert_eq!(notes.status, 200);
    assert_eq!(notes.text(), NOTES_AS_SENT);
}

#[test]
fn notes_are_saved_byte_for_byte_in_the_on_disk_format() {
    let (_dir, root) = common::real_tempdir();
    let doc = root.join("my plans/é plan.md");
    let server = Preview::start();
    let path = format!("/__notes{}", encode(&doc));

    let put = server.request(
        "PUT",
        &path,
        &[("Content-Type", "application/json")],
        Some(NOTES_AS_SENT),
    );

    assert_eq!(put.status, 204);
    let file = server
        .notes_dir()
        .join(format!("{}.json", sha1_hex(doc.to_str().unwrap())));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), notes_file(&doc));
    assert_eq!(server.get(&path).text(), NOTES_AS_SENT);

    let cleared = server.request(
        "PUT",
        &path,
        &[("Content-Type", "application/json")],
        Some(r#"{"notes":[]}"#),
    );
    assert_eq!(cleared.status, 204);
    assert!(!file.exists());
    assert_eq!(server.get(&path).text(), r#"{"notes":[]}"#);
}

#[test]
fn copy_all_is_the_page_path_then_one_line_per_range_in_line_order() {
    let server = Preview::start();

    let app = server.get("/__app.js").text();

    for line in [
        "const file = decodeURIComponent(location.pathname)",
        "return n.l0 === n.l1 ? 'L' + n.l0 : 'L' + n.l0 + '-L' + n.l1",
        ".sort((a, b) => a.l0 - b.l0)",
        ".map((n) => label(n) + (n.orphan ? ' (orphaned)' : '') + ': ' + n.text.replace(/\\n/g, ' '))",
        "const out = file + '\\n\\n' + body + '\\n'",
    ] {
        assert!(app.contains(line), "app.js lost: {line}");
    }
}

#[test]
fn the_page_scrolls_to_the_cursor_only_when_it_leaves_the_middle_of_the_viewport() {
    let server = Preview::start();

    let app = server.get("/__app.js").text();

    for line in [
        "es.addEventListener('cursor', (ev) => follow(JSON.parse(ev.data).line))",
        "if (r.top >= h * 0.15 && r.top <= h * 0.75) return",
        "el.scrollIntoView({ block: 'center', behavior: 'smooth' })",
    ] {
        assert!(app.contains(line), "app.js lost: {line}");
    }
}

#[test]
fn a_write_without_a_json_content_type_is_refused() {
    let (_dir, root) = common::real_tempdir();
    let doc = root.join("plan.md");
    write(&doc, "hi");
    let server = Preview::start();

    let form = server.request(
        "PUT",
        &format!("/__notes{}", encode(&doc)),
        &[("Content-Type", "text/plain")],
        Some(r#"{"notes":[{"id":"n1"}]}"#),
    );
    let cursor = server.request(
        "POST",
        &format!("/__cursor{}", encode(&doc)),
        &[("Content-Type", "application/x-www-form-urlencoded")],
        Some(r#"{"line":3}"#),
    );

    assert_eq!(form.status, 400);
    assert_eq!(cursor.status, 400);
    assert!(!server.notes_dir().exists());
}

#[test]
fn notes_are_only_kept_for_markdown_files() {
    let server = Preview::start();

    let response = server.request(
        "PUT",
        "/__notes/etc/passwd",
        &[("Content-Type", "application/json")],
        Some(r#"{"notes":[{"id":"n1"}]}"#),
    );

    assert_eq!(response.status, 400);
}

#[test]
fn saving_the_file_pushes_a_change_event() {
    let (_dir, root) = common::real_tempdir();
    let doc = root.join("live.md");
    write(&doc, "one");
    let server = Preview::start();
    let mut events = server.events(&doc);
    std::thread::sleep(Duration::from_millis(600));

    write(&doc, "two, longer");
    assert_eq!(events.next_event("change"), r#"{"gone":false}"#);

    std::fs::remove_file(&doc).unwrap();
    assert_eq!(events.next_event("change"), r#"{"gone":true}"#);
}

#[test]
fn a_cursor_post_reaches_the_page_and_is_replayed_on_connect() {
    let (_dir, root) = common::real_tempdir();
    let doc = root.join("live.md");
    write(&doc, "one");
    let server = Preview::start();
    let mut events = server.events(&doc);
    let cursor = format!("/__cursor{}", encode(&doc));

    let posted = server.request(
        "POST",
        &cursor,
        &[("Content-Type", "application/json")],
        Some(r#"{"line":12}"#),
    );

    assert_eq!(posted.status, 204);
    assert_eq!(events.next_event("cursor"), r#"{"line":12}"#);
    assert_eq!(server.events(&doc).next_event("cursor"), r#"{"line":12}"#);
    let bad = server.request(
        "POST",
        &cursor,
        &[("Content-Type", "application/json")],
        Some(r#"{"line":0}"#),
    );
    assert_eq!(bad.status, 400);
}

#[test]
fn the_server_restarts_from_its_binary_when_it_is_replaced() {
    let (_dir, root) = common::real_tempdir();
    let binary = root.join("atelier");
    std::fs::copy(env!("CARGO_BIN_EXE_atelier"), &binary).unwrap();
    let mut server = Preview::start_with(&binary);
    let marker = root.join("restarted");

    std::fs::remove_file(&binary).unwrap();
    std::fs::write(
        &binary,
        format!(
            "#!/bin/sh\necho \"$@\" >'{}'\nexec '{}' \"$@\"\n",
            marker.display(),
            env!("CARGO_BIN_EXE_atelier")
        ),
    )
    .unwrap();
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).unwrap();

    common::wait_until("the server to restart", || {
        marker.exists() && alive(server.port)
    });
    assert_eq!(std::fs::read_to_string(&marker).unwrap(), "preview serve\n");
    assert!(
        server.child.try_wait().unwrap().is_none(),
        "it restarted in place"
    );
    assert_eq!(server.get("/__meta").status, 200);
}

fn open(port: u16, home: &Path, file: &Path) -> Output {
    open_with(&common::FakeOpener::new(), port, home, file)
}

fn open_with(opener: &common::FakeOpener, port: u16, home: &Path, file: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_atelier"))
        .arg("preview")
        .arg(file)
        .env("HOME", home)
        .env("MD_PREVIEW_PORT", port.to_string())
        .env("PATH", opener.path())
        .bounded_output()
        .expect("atelier runs")
}

#[test]
fn opening_a_file_hands_its_url_to_the_os_opener() {
    let (_dir, root) = common::real_tempdir();
    let doc = root.join("plan.md");
    write(&doc, "hi");
    let server = Preview::start();
    let opener = common::FakeOpener::new();

    let output = open_with(&opener, server.port, server.home.path(), &doc);

    assert!(output.status.success());
    assert_eq!(
        opener.wait_opened(),
        common::FakeOpener::browser_call(&server.url(&doc))
    );
}

#[test]
fn opening_a_file_prints_its_url() {
    let (_dir, root) = common::real_tempdir();
    let doc = root.join("a plan.md");
    write(&doc, "hi");
    let server = Preview::start();

    let output = open(server.port, server.home.path(), &doc);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!("{}\n", server.url(&doc))
    );
}

#[test]
fn opening_a_file_starts_the_server_when_none_is_running() {
    let (_dir, root) = common::real_tempdir();
    let doc = root.join("plan.md");
    write(&doc, "hi");
    let port = common::free_port();

    let output = open(port, &root, &doc);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("server is up");
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    write!(
        stream,
        "GET /__meta HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).unwrap();
    let meta = String::from_utf8(parse(&raw).body).unwrap();
    let pid = meta
        .split(r#""pid":"#)
        .nth(1)
        .unwrap()
        .trim_end_matches('}')
        .to_string();
    Command::new("kill").arg(&pid).status().unwrap();
}

#[test]
fn opening_refuses_what_is_not_an_existing_markdown_file() {
    let (_dir, root) = common::real_tempdir();
    write(&root.join("notes.txt"), "hi");
    let port = common::free_port();

    let text = open(port, &root, &root.join("notes.txt"));
    let missing = open(port, &root, &root.join("missing.md"));

    assert!(!text.status.success());
    assert!(String::from_utf8_lossy(&text.stderr).contains("not a markdown file"));
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("no such file"));
    assert!(!alive(port));
}

#[test]
fn a_symlinked_markdown_page_reaches_only_the_repo_it_sits_in() {
    let (_dir, root) = common::real_tempdir();
    let repo = root.join("repo");
    common::git_repo(&repo);
    write(&root.join("keys/notes.md"), "hi");
    write(&root.join("keys/id_rsa"), "PRIVATE");
    std::os::unix::fs::symlink(root.join("keys/notes.md"), repo.join("notes.md")).unwrap();
    let server = Preview::start();

    let response = server.request(
        "GET",
        &encode(&root.join("keys/id_rsa")),
        &[("Referer", &server.url(&repo.join("notes.md")))],
        None,
    );

    assert_eq!(response.status, 403);
}

#[test]
fn a_hidden_file_or_folder_is_never_served_to_a_page() {
    let (_dir, root) = common::real_tempdir();
    let repo = root.join("repo");
    common::git_repo(&repo);
    write(&repo.join("README.md"), "hi");
    write(&repo.join(".env"), "TOKEN=1");
    write(&repo.join(".git/config"), "[core]");
    write(&root.join("README.md"), "hi");
    write(&root.join(".netrc"), "machine x password y");
    let server = Preview::start();
    let get = |file: &Path, page: &Path| {
        server
            .request(
                "GET",
                &encode(file),
                &[("Referer", &server.url(page))],
                None,
            )
            .status
    };

    assert_eq!(
        [
            get(&repo.join(".env"), &repo.join("README.md")),
            get(&repo.join(".git/config"), &repo.join("README.md")),
            get(&root.join(".netrc"), &root.join("README.md")),
        ],
        [403, 403, 403]
    );
}

#[test]
fn raw_refuses_a_markdown_name_that_links_to_another_kind_of_file() {
    let (_dir, root) = common::real_tempdir();
    write(&root.join("credentials"), "SECRET");
    std::os::unix::fs::symlink(root.join("credentials"), root.join("notes.md")).unwrap();
    let server = Preview::start();

    let response = server.get(&format!("/__raw{}", encode(&root.join("notes.md"))));

    assert_eq!(response.status, 404);
}

#[test]
fn events_are_only_streamed_for_an_existing_markdown_file() {
    let (_dir, root) = common::real_tempdir();
    write(&root.join("data.json"), "{}");
    let server = Preview::start();

    let other = server.get(&format!("/__events{}", encode(&root.join("data.json"))));
    let missing = server.get(&format!("/__events{}", encode(&root.join("gone.md"))));

    assert_eq!((other.status, missing.status), (404, 404));
}

#[test]
fn concurrent_saves_of_the_same_notes_all_succeed() {
    let (_dir, root) = common::real_tempdir();
    let doc = root.join("plan.md");
    write(&doc, "hi");
    let server = Preview::start();
    let path = format!("/__notes{}", encode(&doc));

    let statuses: Vec<u16> = std::thread::scope(|scope| {
        let saves: Vec<_> = (0..20)
            .map(|n| {
                let (server, path) = (&server, &path);
                scope.spawn(move || {
                    let body = format!(r#"{{"notes":[{{"id":"n{n}"}}]}}"#);
                    server
                        .request(
                            "PUT",
                            path,
                            &[("Content-Type", "application/json")],
                            Some(&body),
                        )
                        .status
                })
            })
            .collect();
        saves.into_iter().map(|save| save.join().unwrap()).collect()
    });

    assert_eq!(statuses, vec![204; 20]);
}

#[test]
fn requests_waiting_on_a_hung_git_leave_the_server_answering() {
    let (_dir, root) = common::real_tempdir();
    write(&root.join("docs/plan.md"), "hi");
    write(&root.join("docs/a.png"), "PNG");
    let bin = root.join("bin");
    write(&bin.join("git"), "#!/bin/sh\nexec /bin/sleep 30\n");
    std::fs::set_permissions(bin.join("git"), std::fs::Permissions::from_mode(0o755)).unwrap();
    let server =
        Preview::start_with_path(Path::new(env!("CARGO_BIN_EXE_atelier")), bin.as_os_str());
    let image = encode(&root.join("docs/a.png"));
    let referer = server.url(&root.join("docs/plan.md"));

    let waiting: Vec<_> = (0..32)
        .map(|_| {
            let (port, image, referer) = (server.port, image.clone(), referer.clone());
            std::thread::spawn(move || {
                let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
                let _ = write!(
                    stream,
                    "GET {image} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nReferer: {referer}\r\n\r\n"
                );
                std::thread::sleep(Duration::from_secs(3));
            })
        })
        .collect();
    std::thread::sleep(Duration::from_millis(500));
    let started = std::time::Instant::now();
    let meta = server.get("/__meta");

    assert_eq!(meta.status, 200);
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "answered after {:?}",
        started.elapsed()
    );
    for request in waiting {
        request.join().unwrap();
    }
}
