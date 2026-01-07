use std::path::{Path, PathBuf};
use std::process::Command;

use color_eyre::eyre::{self, WrapErr};
use owo_colors::{OwoColorize, Stream};

use crate::{
    Repo,
    commands::list::{find_worktrees, format_worktree},
    editor::{launch_worktree, resolve_editor_preference, EditorPreferenceResolution},
    telemetry::{EditorLaunchStatus, log_editor_launch_attempt},
};

pub struct OpenCommand {
    name: Option<String>,
    path: Option<PathBuf>,
}

impl OpenCommand {
    pub fn new(name: Option<String>, path: Option<PathBuf>) -> Self {
        Self { name, path }
    }

    pub fn execute(&self, repo: &Repo) -> color_eyre::Result<()> {
        let resolved = self.resolve_target(repo)?;

        // Check if we're in a tmux session
        if std::env::var("TMUX").is_ok() {
            return self.execute_tmux(repo, &resolved);
        }

        self.execute_direct(repo, &resolved)
    }

    fn execute_direct(&self, repo: &Repo, resolved: &ResolvedWorktree) -> color_eyre::Result<()> {
        let outcome = match launch_worktree(repo, &resolved.name, &resolved.path, false) {
            Ok(outcome) => {
                log_editor_launch_attempt(
                    &resolved.name,
                    &resolved.path,
                    outcome.status,
                    &outcome.message,
                );
                outcome
            }
            Err(error) => {
                log_editor_launch_attempt(
                    &resolved.name,
                    &resolved.path,
                    EditorLaunchStatus::ConfigurationError,
                    &error.to_string(),
                );
                return Err(error);
            }
        };

        match outcome.status {
            EditorLaunchStatus::Success => {
                println!(
                    "Opened `{}` at `{}`.",
                    resolved.name,
                    resolved.path.display()
                );
                println!("{}", outcome.message);
                Ok(())
            }
            EditorLaunchStatus::PreferenceMissing => {
                println!("{}", outcome.message);
                Ok(())
            }
            _ => {
                eprintln!("{}", outcome.message);
                Err(eyre::eyre!(outcome.message))
            }
        }
    }

    fn execute_tmux(&self, repo: &Repo, resolved: &ResolvedWorktree) -> color_eyre::Result<()> {
        let project_name = repo
            .root()
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let window_name = format!("{}/{}", project_name, resolved.name);

        // Get the editor command
        let editor_command = match resolve_editor_preference(repo)? {
            EditorPreferenceResolution::Found(pref) => {
                pref.command.to_string_lossy().into_owned()
            }
            EditorPreferenceResolution::Missing(reason) => {
                return Err(eyre::eyre!("No editor configured: {:?}", reason));
            }
        };

        // Check if we're in the worktree window
        let current_window = Command::new("tmux")
            .args(["display-message", "-p", "#{window_name}"])
            .output()
            .wrap_err("failed to get current tmux window name")?;

        let current_window_name = String::from_utf8_lossy(&current_window.stdout)
            .trim()
            .to_string();

        if current_window_name == window_name {
            // We're in the worktree window, check for editor pane
            if let Some(pane_id) = self.find_editor_pane(&editor_command)? {
                // Select the existing editor pane
                let status = Command::new("tmux")
                    .args(["select-pane", "-t", &pane_id])
                    .status()
                    .wrap_err("failed to select tmux pane")?;

                if !status.success() {
                    return Err(eyre::eyre!("failed to select editor pane"));
                }

                let pane_label = format_with_color(&pane_id, |text| {
                    format!("{}", text.cyan().bold())
                });
                println!("Switched to editor pane `{}`", pane_label);
                return Ok(());
            }

            // No editor pane found, create a new one
            return self.create_editor_pane(repo, resolved, &editor_command);
        }

        // Check if the worktree window exists
        let list_output = Command::new("tmux")
            .args(["list-windows", "-F", "#{window_name}"])
            .output()
            .wrap_err("failed to list tmux windows")?;

        let existing_windows = String::from_utf8_lossy(&list_output.stdout);
        let window_exists = existing_windows
            .lines()
            .any(|line| line.trim() == window_name);

        if window_exists {
            // Switch to the window first
            let status = Command::new("tmux")
                .args(["select-window", "-t", &window_name])
                .status()
                .wrap_err("failed to switch to tmux window")?;

            if !status.success() {
                return Err(eyre::eyre!("failed to switch to tmux window `{}`", window_name));
            }

            // Now check for editor pane in that window
            if let Some(pane_id) = self.find_editor_pane_in_window(&window_name, &editor_command)? {
                let status = Command::new("tmux")
                    .args(["select-pane", "-t", &pane_id])
                    .status()
                    .wrap_err("failed to select tmux pane")?;

                if !status.success() {
                    return Err(eyre::eyre!("failed to select editor pane"));
                }

                let window_label = format_with_color(&window_name, |text| {
                    format!("{}", text.cyan().bold())
                });
                println!("Switched to editor in window `{}`", window_label);
                return Ok(());
            }

            // No editor pane, create one
            return self.create_editor_pane(repo, resolved, &editor_command);
        }

        // Window doesn't exist, create it and open editor
        let status = Command::new("tmux")
            .args([
                "new-window",
                "-n",
                &window_name,
                "-c",
                &resolved.path.display().to_string(),
                &editor_command,
                &resolved.path.display().to_string(),
            ])
            .status()
            .wrap_err("failed to create tmux window with editor")?;

        if !status.success() {
            return Err(eyre::eyre!("failed to create tmux window `{}`", window_name));
        }

        let window_label = format_with_color(&window_name, |text| {
            format!("{}", text.cyan().bold())
        });
        println!("Created window `{}` with editor", window_label);
        Ok(())
    }

    fn find_editor_pane(&self, editor_command: &str) -> color_eyre::Result<Option<String>> {
        // List panes in current window with their commands
        let output = Command::new("tmux")
            .args(["list-panes", "-F", "#{pane_id}:#{pane_current_command}"])
            .output()
            .wrap_err("failed to list tmux panes")?;

        let panes = String::from_utf8_lossy(&output.stdout);
        for line in panes.lines() {
            if let Some((pane_id, cmd)) = line.split_once(':') {
                if cmd.contains(editor_command) || self.is_editor_command(cmd) {
                    return Ok(Some(pane_id.to_string()));
                }
            }
        }

        Ok(None)
    }

    fn find_editor_pane_in_window(
        &self,
        window_name: &str,
        editor_command: &str,
    ) -> color_eyre::Result<Option<String>> {
        let output = Command::new("tmux")
            .args([
                "list-panes",
                "-t",
                window_name,
                "-F",
                "#{pane_id}:#{pane_current_command}",
            ])
            .output()
            .wrap_err("failed to list tmux panes")?;

        let panes = String::from_utf8_lossy(&output.stdout);
        for line in panes.lines() {
            if let Some((pane_id, cmd)) = line.split_once(':') {
                if cmd.contains(editor_command) || self.is_editor_command(cmd) {
                    return Ok(Some(pane_id.to_string()));
                }
            }
        }

        Ok(None)
    }

    fn is_editor_command(&self, cmd: &str) -> bool {
        let editors = ["vim", "nvim", "nano", "emacs", "code", "cursor", "webstorm", "rider", "idea"];
        editors.iter().any(|e| cmd.contains(e))
    }

    fn create_editor_pane(
        &self,
        repo: &Repo,
        resolved: &ResolvedWorktree,
        editor_command: &str,
    ) -> color_eyre::Result<()> {
        // Get editor args if any
        let editor_args = match resolve_editor_preference(repo)? {
            EditorPreferenceResolution::Found(pref) => {
                pref.args.iter()
                    .map(|a| a.to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
            }
            _ => Vec::new(),
        };

        // Build the full command
        let mut cmd_parts = vec![editor_command.to_string()];
        cmd_parts.extend(editor_args);
        cmd_parts.push(resolved.path.display().to_string());
        let full_cmd = cmd_parts.join(" ");

        // Create a new pane with the editor
        let status = Command::new("tmux")
            .args([
                "split-window",
                "-h",
                "-c",
                &resolved.path.display().to_string(),
                &full_cmd,
            ])
            .status()
            .wrap_err("failed to create tmux pane with editor")?;

        if !status.success() {
            return Err(eyre::eyre!("failed to create editor pane"));
        }

        let editor_label = format_with_color(editor_command, |text| {
            format!("{}", text.cyan().bold())
        });
        println!("Opened `{}` in new pane", editor_label);
        Ok(())
    }

    fn resolve_target(&self, repo: &Repo) -> color_eyre::Result<ResolvedWorktree> {
        if let Some(path) = &self.path {
            return resolve_by_path(path, repo);
        }

        let name = self
            .name
            .as_ref()
            .ok_or_else(|| eyre::eyre!("worktree name or --path must be provided"))?;
        resolve_by_name(name, repo)
    }
}

fn format_with_color(value: &str, paint: impl Fn(&str) -> String) -> String {
    value
        .if_supports_color(Stream::Stdout, |text| paint(text))
        .to_string()
}

struct ResolvedWorktree {
    name: String,
    path: PathBuf,
}

fn resolve_by_name(name: &str, repo: &Repo) -> color_eyre::Result<ResolvedWorktree> {
    let worktrees_dir = repo.ensure_worktrees_dir()?;
    let entries = find_worktrees(&worktrees_dir)?;

    let mut matches = Vec::new();

    for rel in entries {
        let display = format_worktree(&rel);
        let file_name = rel
            .file_name()
            .map(|component| component.to_string_lossy().into_owned());

        let is_match = display == name
            || display.ends_with(&format!("/{name}"))
            || file_name.as_deref() == Some(name);

        if is_match {
            matches.push((display, rel));
        }
    }

    if matches.is_empty() {
        return Err(eyre::eyre!(
            "worktree `{}` not found. Run `rsworktree ls` to view available worktrees.",
            name
        ));
    }

    if matches.len() > 1 {
        let names = matches
            .iter()
            .map(|(display, _)| display.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(eyre::eyre!(
            "worktree identifier `{}` is ambiguous. Matches: {}",
            name,
            names
        ));
    }

    let (display, rel) = matches.into_iter().next().unwrap();
    let absolute = worktrees_dir.join(&rel);

    if !absolute.exists() {
        return Err(eyre::eyre!(
            "worktree `{}` is missing from `{}`",
            display,
            absolute.display()
        ));
    }

    let canonical = absolute
        .canonicalize()
        .wrap_err_with(|| eyre::eyre!("failed to resolve `{}`", absolute.display()))?;

    Ok(ResolvedWorktree {
        name: display,
        path: canonical,
    })
}

fn resolve_by_path(path: &Path, repo: &Repo) -> color_eyre::Result<ResolvedWorktree> {
    if !path.exists() {
        return Err(eyre::eyre!(
            "worktree path `{}` does not exist",
            path.display()
        ));
    }

    let canonical = path
        .canonicalize()
        .wrap_err_with(|| eyre::eyre!("failed to resolve `{}`", path.display()))?;

    let worktrees_dir = repo.ensure_worktrees_dir()?;
    let display = if let Ok(relative) = canonical.strip_prefix(&worktrees_dir) {
        format_worktree(relative)
    } else if let Some(name) = canonical.file_name().and_then(|n| n.to_str()) {
        name.to_string()
    } else {
        canonical.display().to_string()
    };

    Ok(ResolvedWorktree {
        name: display,
        path: canonical,
    })
}
