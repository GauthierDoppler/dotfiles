use std::collections::HashSet;

const URL_SCHEMES: [&str; 3] = ["https://", "http://", "ftp://"];

pub(super) fn is_url(token: &str) -> bool {
    URL_SCHEMES.iter().any(|scheme| token.starts_with(scheme)) || token.starts_with("git@")
}

pub(super) fn candidates(capture: &str) -> (Vec<String>, Vec<String>) {
    let mut urls = Vec::new();
    let mut paths = Vec::new();
    let mut seen = HashSet::new();
    for line in capture.lines().rev() {
        for url in urls_in(line) {
            if url.chars().count() > 6 && seen.insert(url.clone()) {
                urls.push(url);
            }
        }
        for path in paths_in(line) {
            if path.chars().count() > 3 && seen.insert(path.clone()) {
                paths.push(path);
            }
        }
    }
    (urls, paths)
}

fn trim_punctuation(token: &str) -> &str {
    token.trim_end_matches(['.', ',', ':', ';'])
}

fn url_start(rest: &str) -> Option<usize> {
    if let Some(scheme) = URL_SCHEMES.iter().find(|s| rest.starts_with(**s)) {
        return Some(scheme.len());
    }
    let host = rest.strip_prefix("git@")?;
    let len = host
        .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')))
        .unwrap_or(host.len());
    (len > 0 && host[len..].starts_with(':')).then_some(4 + len + 1)
}

fn urls_in(line: &str) -> Vec<String> {
    let mut urls = Vec::new();
    let mut at = 0;
    while at < line.len() {
        let rest = &line[at..];
        let Some(prefix) = url_start(rest) else {
            at += rest.chars().next().map_or(1, char::len_utf8);
            continue;
        };
        let tail = &rest[prefix..];
        let len = tail
            .find(|c: char| c.is_whitespace() || "<>\"'`|(),".contains(c))
            .unwrap_or(tail.len());
        if len == 0 {
            at += prefix;
            continue;
        }
        let url = trim_punctuation(&rest[..prefix + len]);
        if !url.contains('…') {
            urls.push(url.to_string());
        }
        at += prefix + len;
    }
    urls
}

fn is_path_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '~' | '@' | '+' | '-')
}

fn paths_in(line: &str) -> Vec<String> {
    line.split(|c: char| !(is_path_char(c) || c == '/'))
        .flat_map(split_on_double_slash)
        .filter_map(path_from_piece)
        .collect()
}

fn split_on_double_slash(run: &str) -> Vec<&str> {
    let mut pieces = Vec::new();
    let mut start = 0;
    while let Some(found) = run[start..].find("//") {
        let at = start + found;
        pieces.push(&run[start..at]);
        let slashes = run[at..].len() - run[at..].trim_start_matches('/').len();
        start = at + slashes - 1;
    }
    pieces.push(&run[start..]);
    pieces
}

fn path_from_piece(piece: &str) -> Option<String> {
    let piece = trim_punctuation(piece.trim_end_matches('/'));
    piece
        .strip_prefix('/')
        .unwrap_or(piece)
        .contains('/')
        .then(|| piece.to_string())
}
