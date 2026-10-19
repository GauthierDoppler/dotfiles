---
name: add-config
description: Add a new config file or folder to the dotfiles repo with symlinks and install script update.
argument-hint: [path]
allowed-tools: Read, Glob, Grep, Bash, Edit, Write
---

Add the config at `$ARGUMENTS` to the dotfiles repo.

## Rules

1. Determine the type:
   - **Folder in `~/.config/`**: symlink the whole folder
   - **Dotfile in `~/`**: copy as `dot_<name>` (e.g. `~/.bashrc` → `dot_bashrc`)
   - **File in `~/.config/`**: keep parent structure (e.g. `git/ignore`)

2. Steps to follow:
   - Copy the file/folder to `~/dotfiles/`
   - Remove the original
   - Create a symlink from the original location to the dotfiles copy
   - If it's a folder with auto-generated files, add a `.gitignore` inside it
   - Add a `[[link]]` entry to `links.toml`
   - Stage and show the result, but do NOT commit

3. `links.toml` format (`source` relative to the repo, `dest` relative to `$HOME`):
   - Folders: `source = "foldername"`, `dest = ".config/foldername"`
   - Individual files: `source = "path/file"`, `dest = ".config/path/file"`
   - Home dotfiles: `source = "dot_name"`, `dest = ".name"`
   - A GUI app's config, skipped on the `remote` profile: add `desktop_only = true`
   - Each `*.md` of a folder, linked one by one: `each_markdown_in = "folder"`
     instead of `source`

4. Run `atelier setup --dry-run` to check the plan, then `atelier setup`.
