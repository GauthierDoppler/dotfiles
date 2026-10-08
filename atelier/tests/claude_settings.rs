mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{json, Value};

use common::BoundedOutput;

const BASE: &str = "dot_claude/settings.json";
const LOCAL: &str = "dot_claude/settings.local.json";
const SNAPSHOT: &str = "dot_claude/settings.generated.json";
const LIVE: &str = ".claude/settings.json";

struct Machine {
    home: tempfile::TempDir,
    repo: tempfile::TempDir,
}

impl Machine {
    fn new(base: Value) -> Self {
        let machine = Machine {
            home: tempfile::tempdir().unwrap(),
            repo: tempfile::tempdir().unwrap(),
        };
        write(&machine.repo.path().join("dot_zshrc"), "");
        write(&machine.repo.path().join("atelier/Cargo.toml"), "");
        machine.set_base(base);
        machine
    }

    fn set_base(&self, base: Value) {
        write(&self.repo.path().join(BASE), &base.to_string());
    }

    fn set_local(&self, local: Value) {
        write(&self.repo.path().join(LOCAL), &local.to_string());
    }

    fn app_writes(&self, content: &str) {
        write(&self.home.path().join(LIVE), content);
    }

    fn path(&self, file: &str) -> PathBuf {
        if file.starts_with(".claude/") {
            self.home.path().join(file)
        } else {
            self.repo.path().join(file)
        }
    }

    fn read(&self, file: &str) -> Option<String> {
        fs::read_to_string(self.path(file)).ok()
    }

    fn json(&self, file: &str) -> Option<Value> {
        self.read(file)
            .map(|content| serde_json::from_str(&content).unwrap())
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_atelier"))
            .arg("claude-settings-sync")
            .arg("--repo")
            .arg(self.repo.path())
            .args(args)
            .current_dir(self.home.path())
            .env("HOME", self.home.path())
            .env_remove("TMUX")
            .bounded_output()
            .expect("atelier runs")
    }

    fn sync(&self) -> String {
        let output = self.run(&[]);
        assert!(
            output.status.success(),
            "sync failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }

    fn files(&self) -> Vec<Option<String>> {
        [BASE, LOCAL, SNAPSHOT, LIVE]
            .iter()
            .map(|file| self.read(file))
            .chain([self.read(".claude/settings.json.bak")])
            .collect()
    }
}

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

#[test]
fn a_first_run_with_no_live_file_writes_the_base() {
    let machine = Machine::new(json!({"theme": "dark", "env": {"A": "1"}}));

    let output = machine.sync();

    let live = machine.path(LIVE).display().to_string();
    assert_eq!(output, format!("regenerated: {live}\n"));
    assert_eq!(
        machine.read(LIVE).unwrap(),
        "{\n  \"env\": {\n    \"A\": \"1\"\n  },\n  \"theme\": \"dark\"\n}\n"
    );
    assert_eq!(machine.read(SNAPSHOT), machine.read(LIVE));
    assert_eq!(machine.read(LOCAL), None);
}

#[test]
fn a_first_run_captures_what_the_live_file_has_beyond_the_base() {
    let machine = Machine::new(json!({
        "theme": "dark",
        "statusLine": {"command": "$HOME/.claude/statusline.sh", "type": "command"}
    }));
    machine.app_writes(
        r#"{"theme": "dark", "gitAi": true,
            "statusLine": {"command": "/Users/me/.claude/statusline.sh", "type": "command"}}"#,
    );

    machine.sync();

    assert_eq!(
        machine.json(LOCAL),
        Some(json!({
            "gitAi": true,
            "statusLine": {"command": "/Users/me/.claude/statusline.sh"}
        }))
    );
    assert_eq!(
        machine.json(LIVE),
        Some(json!({
            "theme": "dark",
            "gitAi": true,
            "statusLine": {"command": "/Users/me/.claude/statusline.sh", "type": "command"}
        }))
    );
}

#[test]
fn the_app_reordering_keys_captures_nothing() {
    let machine = Machine::new(json!({"a": 1, "b": {"x": true, "y": [1, 2]}}));
    machine.sync();
    let reordered = r#"{"b": {"y": [1, 2], "x": true}, "a": 1}"#;
    machine.app_writes(reordered);

    let output = machine.sync();

    assert_eq!(machine.read(LOCAL), None);
    assert_eq!(machine.read(LIVE).unwrap(), reordered);
    assert_eq!(
        output,
        format!("up to date: {}\n", machine.path(LIVE).display())
    );
}

#[test]
fn the_app_absolutising_a_path_is_captured_and_kept() {
    let machine = Machine::new(json!({"hooks": {"Stop": "$HOME/.git-ai/bin/git-ai"}}));
    machine.sync();
    machine.app_writes(r#"{"hooks": {"Stop": "/Users/me/.git-ai/bin/git-ai"}}"#);

    machine.sync();

    let absolute = json!({"hooks": {"Stop": "/Users/me/.git-ai/bin/git-ai"}});
    assert_eq!(machine.json(LOCAL), Some(absolute.clone()));
    assert_eq!(machine.json(LIVE), Some(absolute));
}

#[test]
fn a_base_change_propagates_where_the_app_was_silent() {
    let machine = Machine::new(json!({"model": "opus", "effort": "medium"}));
    machine.sync();
    machine.app_writes(r#"{"model": "opus", "effort": "high"}"#);
    machine.sync();
    machine.set_base(json!({"model": "sonnet", "effort": "low"}));

    machine.sync();

    assert_eq!(
        machine.json(LIVE),
        Some(json!({"model": "sonnet", "effort": "high"}))
    );
    assert_eq!(machine.json(LOCAL), Some(json!({"effort": "high"})));
}

#[test]
fn a_nested_change_captures_only_the_leaf_and_its_siblings_follow_the_base() {
    let machine = Machine::new(json!({"env": {"A": "1", "B": "2"}}));
    machine.sync();
    machine.app_writes(r#"{"env": {"A": "9", "B": "2"}}"#);
    machine.sync();
    machine.set_base(json!({"env": {"A": "1", "B": "3", "C": "4"}}));

    machine.sync();

    assert_eq!(machine.json(LOCAL), Some(json!({"env": {"A": "9"}})));
    assert_eq!(
        machine.json(LIVE),
        Some(json!({"env": {"A": "9", "B": "3", "C": "4"}}))
    );
}

#[test]
fn an_array_the_app_changed_is_captured_whole_and_replaces_the_base_array() {
    let machine = Machine::new(json!({"permissions": {"allow": ["Read"]}}));
    machine.sync();
    machine.app_writes(r#"{"permissions": {"allow": ["Read", "Bash(ls)"]}}"#);
    machine.sync();
    machine.set_base(json!({"permissions": {"allow": ["Read", "Grep"]}}));

    machine.sync();

    let captured = json!({"permissions": {"allow": ["Read", "Bash(ls)"]}});
    assert_eq!(machine.json(LOCAL), Some(captured.clone()));
    assert_eq!(machine.json(LIVE), Some(captured));
}

#[test]
fn a_key_the_app_removed_is_not_captured_and_comes_back_from_the_base() {
    let machine = Machine::new(json!({"a": 1, "b": 2}));
    machine.sync();
    machine.app_writes(r#"{"a": 1}"#);

    machine.sync();

    assert_eq!(machine.read(LOCAL), None);
    assert_eq!(machine.json(LIVE), Some(json!({"a": 1, "b": 2})));
}

#[test]
fn a_value_the_app_set_to_null_is_captured_as_null() {
    let machine = Machine::new(json!({"a": 1, "b": {"c": 2}}));
    machine.sync();
    machine.app_writes(r#"{"a": null, "b": {"c": 2}}"#);

    machine.sync();

    assert_eq!(machine.json(LOCAL), Some(json!({"a": null})));
    assert_eq!(machine.json(LIVE), Some(json!({"a": null, "b": {"c": 2}})));
}

#[test]
fn an_object_the_app_replaced_with_a_scalar_is_captured_as_that_scalar() {
    let machine = Machine::new(json!({"a": {"b": 1}}));
    machine.sync();
    machine.app_writes(r#"{"a": false}"#);

    machine.sync();

    assert_eq!(machine.json(LOCAL), Some(json!({"a": false})));
    assert_eq!(machine.json(LIVE), Some(json!({"a": false})));
}

#[test]
fn drift_is_merged_into_existing_local_overrides() {
    let machine = Machine::new(json!({"a": 1, "env": {"X": "1"}}));
    machine.set_local(json!({"env": {"Y": "2"}, "keep": true}));
    machine.sync();
    machine.app_writes(r#"{"a": 1, "keep": true, "env": {"X": "1", "Y": "2", "Z": "3"}}"#);

    let output = machine.sync();

    assert_eq!(
        machine.json(LOCAL),
        Some(json!({"env": {"Y": "2", "Z": "3"}, "keep": true}))
    );
    assert_eq!(
        machine.read(LOCAL).unwrap(),
        "{\n  \"env\": {\n    \"Y\": \"2\",\n    \"Z\": \"3\"\n  },\n  \"keep\": true\n}\n"
    );
    assert!(output.starts_with(&format!("updated: {}\n", machine.path(LOCAL).display())));
}

#[test]
fn a_regenerated_live_file_is_backed_up_first() {
    let machine = Machine::new(json!({"a": 1}));
    machine.sync();
    machine.set_base(json!({"a": 2}));

    machine.sync();

    assert_eq!(
        machine.read(".claude/settings.json.bak").unwrap(),
        "{\n  \"a\": 1\n}\n"
    );
    assert_eq!(machine.json(LIVE), Some(json!({"a": 2})));
}

#[test]
fn a_second_run_changes_nothing() {
    let machine = Machine::new(json!({"a": 1}));
    machine.app_writes(r#"{"a": 1, "b": 2}"#);
    machine.sync();
    let after_first = machine.files();
    let mtimes: Vec<_> = [LOCAL, SNAPSHOT, LIVE]
        .iter()
        .map(|file| {
            fs::metadata(machine.path(file))
                .unwrap()
                .modified()
                .unwrap()
        })
        .collect();

    let output = machine.sync();

    assert_eq!(machine.files(), after_first);
    let again: Vec<_> = [LOCAL, SNAPSHOT, LIVE]
        .iter()
        .map(|file| {
            fs::metadata(machine.path(file))
                .unwrap()
                .modified()
                .unwrap()
        })
        .collect();
    assert_eq!(again, mtimes);
    assert_eq!(
        output,
        format!("up to date: {}\n", machine.path(LIVE).display())
    );
}

#[test]
fn an_unreadable_snapshot_falls_back_to_diffing_against_the_base() {
    let machine = Machine::new(json!({"a": 1}));
    write(&machine.path(SNAPSHOT), "not json");
    machine.app_writes(r#"{"a": 1, "b": 2}"#);

    machine.sync();

    assert_eq!(machine.json(LOCAL), Some(json!({"b": 2})));
    assert_eq!(machine.json(SNAPSHOT), Some(json!({"a": 1, "b": 2})));
}

#[test]
fn a_dry_run_shows_what_would_be_captured_and_touches_nothing() {
    let machine = Machine::new(json!({"a": 1}));
    machine.set_local(json!({"c": 3}));
    machine.app_writes(r#"{"a": 1, "b": 2}"#);
    let before = machine.files();

    let output = machine.run(&["--dry-run"]);

    assert!(output.status.success());
    assert_eq!(machine.files(), before);
    let live = machine.path(LIVE).display().to_string();
    let local = machine.path(LOCAL).display().to_string();
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!(
            "(no snapshot yet — diffing against the base)\n\
             ── drift captured from {live} ──\n\
             {{\n  \"b\": 2\n}}\n\
             ── resulting {local} ──\n\
             {{\n  \"b\": 2,\n  \"c\": 3\n}}\n\
             ── resulting {live} ──\n\
             {{\n  \"a\": 1,\n  \"b\": 2,\n  \"c\": 3\n}}\n"
        )
    );
}

#[test]
fn an_invalid_live_file_is_refused_and_nothing_is_written() {
    let machine = Machine::new(json!({"a": 1}));
    machine.app_writes("{ half written");
    let before = machine.files();

    let output = machine.run(&[]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("refusing to touch it"));
    assert_eq!(machine.files(), before);
}

#[test]
fn a_missing_base_is_an_error() {
    let machine = Machine::new(json!({}));
    fs::remove_file(machine.path(BASE)).unwrap();

    let output = machine.run(&[]);

    assert!(!output.status.success());
    assert_eq!(machine.read(LIVE), None);
}
