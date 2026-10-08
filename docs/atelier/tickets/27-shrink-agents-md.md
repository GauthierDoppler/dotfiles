# 27: Shrink AGENTS.md

**What to build:** With the bash gone, rewrite `AGENTS.md` so it records rules and live rationale. Paragraphs that only describe a fixed bug become a pointer to the test that covers it.

**Blocked by:** 06 (Session picker: preview, window cycling and kill), 08 (Task placements: split, popup and detach), 10 (Token picker), 13 (Repo counts follow git without polling), 15 (Markdown notes and cursor sync)

**Status:** done

- [x] Every removed bug paragraph maps to a named test
- [x] Rationale that still applies kept
- [x] Noticeably shorter than today

## Comments

- `AGENTS.md` went from 1136 to 515 lines. Long-form reasoning that still decides
  things but is not needed every session (control-mode mechanics, the repo
  watcher, bar layout and colour history, why the notification banner and bells
  went, project-jump alternatives, the per-file preview server, the nvim
  submodule) moved to `docs/atelier/design-notes.md`, linked from the header and
  from each section that leans on it.
- Pointers are written `file.rs::test_name`, relative to `atelier/tests/`, or
  `src/…rs::test_name` for unit tests in the crate (the bar's render and the
  control-mode parser have no integration test for those cases). Every pointer in
  both files was checked to exist with `grep "fn <name>("`.
- Bug paragraph → test: duplicated `terminal-overrides` →
  `doctor.rs::terminal_features_grown_by_reloads_fail`; per-line grep spawning
  ~6000 processes → `pick.rs::a_full_scrollback_is_listed_without_a_process_per_line`;
  truncated path fragments → `pick.rs::a_claude_code_capture_yields_its_urls_then_the_paths_that_exist`;
  `LANG` over-padding → `bar_right.rs::the_block_is_as_wide_whatever_the_locale_and_the_repo_state`;
  porcelain `git diff` rewriting the index → `bar_right.rs::reading_the_counts_never_rewrites_the_index`;
  flat +2 separator drift → `src/bar/right.rs::the_rendered_block_is_as_wide_as_it_says_in_every_combination`;
  80-column bar dropping the window list → `src/bar/left.rs::below_the_widest_tier_the_padding_gives_way_to_the_list`;
  `api_perf_beef` scoping to a non-existent project → `sessions.rs::a_hand_named_session_that_looks_like_grove_belongs_to_no_project`;
  stale `✗` over a running task → `tasks.rs::rerunning_reuses_the_window_and_resets_the_marker_to_running`;
  `respawn-window` selecting a detached task → `tasks.rs::rerunning_a_detached_task_reuses_its_window_without_selecting_it`;
  nested `display-popup` → `tasks.rs::a_popup_task_picked_in_the_picker_runs_in_the_picker_s_popup`;
  pickers not opening when empty → `sessions.rs::a_project_with_no_other_session_says_so_and_tab_still_reaches_the_others`;
  daemon attach reordering the picker → `daemon.rs::the_daemon_attaching_leaves_the_picker_order_alone`;
  `%exit` on a killed session → `daemon.rs::the_daemon_outlives_the_session_it_watches_through`;
  a window named `%end …` → `src/daemon/control.rs::a_block_line_that_mimics_another_command_number_stays_output`;
  thin `PATH` in hooks → `daemon.rs::the_daemon_runs_when_path_lacks_tmux`;
  crash-looping agent shown running → `doctor.rs::a_crash_looping_launchd_agent_fails_although_it_stays_loaded`;
  root-owned `LaunchAgents` → `service.rs::an_unwritable_launch_agents_folder_is_reported_with_the_fix`;
  preview server not restarting → `preview.rs::the_server_restarts_from_its_binary_when_it_is_replaced`;
  base edits captured as local drift → `claude_settings.rs::a_base_change_propagates_where_the_app_was_silent`.
- Kept as rules with no test, because they are constraints of external software
  that a test here cannot pin: the `|| true` on the Claude hook (Claude Code's
  exit-2 semantics), the `#,` comma escape inside `#{?...}`, tmux truncating the
  expanded status string, the `ZDOTDIR` history/compdump pinning, launchd's
  `bootstrap` race and `pended nondemand spawn` deferral.
- Dropped as pure history with no live rule: the 48-column left block, the
  `.icns` size, the "used to gate on `--check`" picker history, per-tool
  version lists for the vendored preview libraries (they live in
  `atelier/assets/preview/lib/` with their licences).
