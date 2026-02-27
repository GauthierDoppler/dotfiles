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
   - Update `install.sh` with the new `link` entry (keep alphabetical order)
   - Stage and show the result, but do NOT commit

3. `install.sh` format:
   - Folders: `link "foldername"          "$HOME/.config/foldername"`
   - Individual files: `link "path/file"  "$HOME/.config/path/file"`
   - Home dotfiles: `link "dot_name"      "$HOME/.name"`

4. Verify the symlink works before finishing.
