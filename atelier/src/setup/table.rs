pub(super) enum Source {
    Path(&'static str),
    EachMarkdownIn(&'static str),
}

pub(super) struct Link {
    pub(super) source: Source,
    pub(super) dest: &'static str,
    pub(super) desktop_only: bool,
}

const fn link(source: &'static str, dest: &'static str) -> Link {
    Link {
        source: Source::Path(source),
        dest,
        desktop_only: false,
    }
}

const fn desktop(source: &'static str, dest: &'static str) -> Link {
    Link {
        source: Source::Path(source),
        dest,
        desktop_only: true,
    }
}

const fn each_markdown_in(dir: &'static str, dest: &'static str) -> Link {
    Link {
        source: Source::EachMarkdownIn(dir),
        dest,
        desktop_only: false,
    }
}

pub(super) const LINKS: &[Link] = &[
    link("delta", ".config/delta"),
    desktop("ghostty", ".config/ghostty"),
    link("lazydocker", ".config/lazydocker"),
    link("lazygit", ".config/lazygit"),
    link("nvim", ".config/nvim"),
    link("tmux/oneshot", ".config/tmux/oneshot"),
    desktop("zed", ".config/zed"),
    link("git/ignore", ".config/git/ignore"),
    link("dot_tmux.conf", ".tmux.conf"),
    link("dot_claude/CLAUDE.md", ".claude/CLAUDE.md"),
    link(
        "dot_claude/statusline-custom.sh",
        ".claude/statusline-custom.sh",
    ),
    link("dot_claude/hooks", ".claude/hooks"),
    link("dot_claude/skills/tmux-tasks", ".claude/skills/tmux-tasks"),
    link("dot_claude/skills/grove", ".claude/skills/grove"),
    link("dot_claude/skills/notion", ".claude/skills/notion"),
    link(
        "dot_pi_agent/extensions/subagents",
        ".pi/agent/extensions/subagents",
    ),
    each_markdown_in("dot_pi_agent/agents", ".pi/agent/agents"),
    link("scripts/ssh-setup", ".local/bin/ssh-setup"),
    link("scripts/cc-tap-service", ".local/bin/cc-tap-service"),
];

pub(crate) enum Format {
    Shell,
    Git,
}

pub(crate) struct Stub {
    pub(crate) dest: &'static str,
    pub(crate) shared: &'static str,
    pub(crate) format: Format,
    pub(super) home: &'static str,
    pub(super) load: &'static str,
    pub(super) legacy: Option<&'static str>,
}

pub(crate) const STUBS: &[Stub] = &[
    Stub {
        dest: ".zshrc",
        shared: "dot_zshrc",
        format: Format::Shell,
        home: "$HOME",
        load: "source \"{shared}\"",
        legacy: Some(".zshrc.local"),
    },
    Stub {
        dest: ".zprofile",
        shared: "dot_zprofile",
        format: Format::Shell,
        home: "$HOME",
        load: "source \"{shared}\"",
        legacy: None,
    },
    Stub {
        dest: ".gitconfig",
        shared: "dot_gitconfig",
        format: Format::Git,
        home: "~",
        load: "[include]\n\tpath = {shared}",
        legacy: Some(".gitconfig.local"),
    },
];

pub(super) const STUB_HEADER: &str = "\
# Loads the shared dotfiles config. Everything BELOW this block is
# machine-local and stays out of the dotfiles repo — put local overrides
# here, and let installers append here too.
";
