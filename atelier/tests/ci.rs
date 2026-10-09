#![allow(clippy::disallowed_methods)]

mod common;

const WORKFLOW: &str = include_str!("../../.github/workflows/atelier.yml");

const READ_BY_THE_TESTS_OR_CHECKED: &[&str] = &[
    "dot_tmux.conf",
    "services.toml",
    "install.sh",
    "dot_zshrc",
    "dot_gitconfig",
    "dot_claude/settings.json",
    "dot_claude/statusline-custom.sh",
    "scripts/cc-tap-service",
    "tmux/oneshot/.zshrc",
    "atelier/src/main.rs",
    ".github/workflows/atelier.yml",
];

fn ignored_by(pattern: &str, path: &str) -> bool {
    if let Some(dir) = pattern.strip_suffix("/**") {
        return path.starts_with(&format!("{dir}/"));
    }
    if let Some(extension) = pattern.strip_prefix("*.") {
        return !path.contains('/') && path.ends_with(&format!(".{extension}"));
    }
    panic!("teach this test the glob {pattern:?}");
}

fn ignore_lists() -> Vec<Vec<String>> {
    let mut lists = Vec::new();
    let mut lines = WORKFLOW.lines().peekable();
    while let Some(line) = lines.next() {
        if line.trim() == "paths:" {
            panic!("the workflow filters with `paths`, which misses files the tests read");
        }
        if line.trim() != "paths-ignore:" {
            continue;
        }
        let mut list = Vec::new();
        while let Some(item) = lines.peek().and_then(|next| next.trim().strip_prefix("- ")) {
            list.push(item.trim_matches('\'').to_string());
            lines.next();
        }
        lists.push(list);
    }
    lists
}

#[test]
fn ci_runs_on_every_trigger_for_the_files_the_tests_read() {
    let lists = ignore_lists();
    assert_eq!(lists.len(), 2, "push and pull_request each ignore a list");
    for path in READ_BY_THE_TESTS_OR_CHECKED {
        assert!(common::repo().join(path).exists(), "{path} moved");
        for list in &lists {
            assert!(
                !list.iter().any(|pattern| ignored_by(pattern, path)),
                "{path} is ignored by {list:?}"
            );
        }
    }
}

#[test]
fn shellcheck_covers_the_claude_statusline() {
    assert!(WORKFLOW.lines().any(|line| {
        line.trim_start().starts_with("files=(") && line.contains("dot_claude/statusline-custom.sh")
    }));
}
