# rsworktree

---

This is a fork of [rsworktree](https://github.com/ozankasikci/rust-git-worktree) with support for both Gitlab and Github and some command design changes:

- Rename `pr-github` command to `review`
- Rename `merge-pr-github` command to `merge`

---

[![Crates.io](https://img.shields.io/crates/v/rsworktree.svg)](https://crates.io/crates/rsworktree)
[![Downloads](https://img.shields.io/crates/d/rsworktree.svg)](https://crates.io/crates/rsworktree)
[![License](https://img.shields.io/crates/l/rsworktree.svg)](https://github.com/ozankasikci/rust-git-worktree/blob/master/LICENSE)
[![Codecov](https://codecov.io/gh/ozankasikci/rust-git-worktree/branch/master/graph/badge.svg)](https://codecov.io/gh/ozankasikci/rust-git-worktree)
[![CI](https://github.com/ozankasikci/rust-git-worktree/actions/workflows/coverage.yml/badge.svg)](https://github.com/ozankasikci/rust-git-worktree/actions/workflows/coverage.yml)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-blue.svg)](https://www.rust-lang.org)

`rsworktree` is a Rust CLI for managing Git worktrees in a single repo-local directory (`.rsworktree`). It provides a focused, ergonomic workflow for creating, jumping into, listing, and removing worktrees without leaving the terminal.

## Table of Contents

- [Interactive mode](#interactive-mode)
- [CLI commands](#cli-commands)
  - [`rsworktree create`](#rsworktree-create)
  - [`rsworktree cd`](#rsworktree-cd)
  - [`rsworktree ls`](#rsworktree-ls)
  - [`rsworktree rm`](#rsworktree-rm)
  - [`rsworktree review`](#rsworktree-review)
  - [`rsworktree merge`](#rsworktree-merge)
  - [`rsworktree worktree open-editor`](#rsworktree-worktree-open-editor)
- [Installation](#installation)
- [Environment](#environment)

## Interactive mode

- Open a terminal UI for browsing worktrees, focusing actions, and inspecting details without memorizing subcommands.
- Launch it with the `interactive` command: `rsworktree interactive` (shortcut: `rsworktree i`).
- Available actions include opening worktrees, launching editors, removing worktrees, creating PRs, and merging PRs without leaving the TUI.
- Use the **Open in Editor** action to launch the highlighted worktree in your configured editor (initial support covers `vim`, `cursor`, `webstorm`, and `rider`; see the quickstart for setup guidance).
- The merge flow lets you decide whether to keep the local branch, delete the remote branch, and clean up the worktree before exiting.
- ![Interactive mode screenshot](tapes/gifs/interactive-mode.gif)

## CLI commands

### `rsworktree create`

- Create a new worktree under `.rsworktree/<name>`. Also changes directory to the worktree.
- Demo: ![Create demo](tapes/gifs/create.gif)
- Options:
  - `--base <branch>` — branch from `<branch>` instead of the current git branch.

### `rsworktree cd`

- Spawn an interactive shell rooted in the named worktree.
- **Tmux integration**: When running inside a tmux session, creates a new tmux window (or switches to it if it already exists) named `<project>/<worktree>`.
- Demo: ![CD demo](tapes/gifs/cd.gif)
- Options:
  - `--print` — write the worktree path to stdout without spawning a shell.

### `rsworktree ls`

- List all worktrees tracked under `.rsworktree`, showing nested worktree paths.
- Demo: ![List demo](tapes/gifs/ls.gif)
- Options:
  - _(none)_

### `rsworktree rm`

- Remove the named worktree.
- Demo: ![Remove demo](tapes/gifs/rm.gif)
- Options:
  - `--force` — force removal, mirroring `git worktree remove --force`.

### `rsworktree review`

- Push the worktree branch and create a pull/merge request for the current or named worktree.
- Demo: ![Review demo](tapes/gifs/review.gif)
- Supports both GitHub (`gh pr create`) and GitLab (`glab mr create`).
- Requires the appropriate CLI to be installed:
  - GitHub: [GitHub CLI](https://cli.github.com/) (`gh`)
  - GitLab: [GitLab CLI](https://gitlab.com/gitlab-org/cli) (`glab`)
- Options:
  - `<name>` — optional explicit worktree to operate on; defaults to the current directory.
  - `--provider <provider>` — git provider to use (`github` or `gitlab`); defaults to config or GitHub.
  - `--no-push` — skip pushing the branch before creating the PR/MR.
  - `--draft` — open the PR/MR in draft mode.
  - `--fill` — auto-populate PR/MR metadata from commits.
  - `--web` — open the creation flow in a browser instead of filling via CLI.
  - `--reviewer <login>` — add one or more reviewers by login.
  - `-- <extra args>` — pass additional arguments through to `gh pr create` or `glab mr create`.

### `rsworktree merge`

- Merge the open pull/merge request for the current or named worktree.
- Demo: ![Merge PR demo](tapes/gifs/merge.gif)
- Supports both GitHub (`gh pr merge`) and GitLab (`glab mr merge`).
- Requires the appropriate CLI to be installed (see `review` command above).
- Options:
  - `<name>` — optional explicit worktree to operate on; defaults to the current directory.
  - `--provider <provider>` — git provider to use (`github` or `gitlab`); defaults to config or GitHub.
  - `--remove` — delete the remote branch after a successful merge.

### `rsworktree worktree open-editor`

- Open the specified worktree (or the current directory when omitted) in your configured editor.
- Editor resolution checks the rsworktree config first, then falls back to `$EDITOR` / `$VISUAL`. If no editor is configured, the command prints actionable guidance instead of failing.
- Initial support focuses on `vim`, `cursor`, `webstorm`, and `rider`. For setup instructions and troubleshooting, see `specs/002-i-want-to/quickstart.md`.

## Installation

Install from crates.io with:

```bash
cargo install rsworktree
```

On macOS you can install via Homebrew:

```bash
brew tap ozankasikci/tap
brew install rsworktree
```

After the binary is on your `PATH`, run `rsworktree --help` to explore the available commands.

## Configuration

You can configure rsworktree by creating a `.rsworktree/preferences.json` file in your repository:

```json
{
  "editor": {
    "command": "vim",
    "args": []
  },
  "provider": "gitlab"
}
```

### Provider Configuration

The `provider` field sets the default git provider for `review` and `merge` commands:
- `"github"` (default) — use GitHub CLI (`gh`)
- `"gitlab"` — use GitLab CLI (`glab`)

Provider resolution order:
1. `--provider` CLI flag
2. Config file (`preferences.json`)
3. `RSWORKTREE_PROVIDER` environment variable
4. Default (`github`)

## Environment

- `RSWORKTREE_SHELL` — override the shell used by `rsworktree cd` (falls back to `$SHELL` or `/bin/sh`).
- `RSWORKTREE_PROVIDER` — set the default git provider (`github` or `gitlab`).
