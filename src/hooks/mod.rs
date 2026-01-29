use std::{
    path::{Path, PathBuf},
    process::Command,
};

use color_eyre::eyre::{self, Context};
use owo_colors::{OwoColorize, Stream};

const HOOKS_DIR: &str = "hooks";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookName {
    PostCreate,
}

impl HookName {
    pub fn as_str(&self) -> &'static str {
        match self {
            HookName::PostCreate => "post-create",
        }
    }
}

impl std::fmt::Display for HookName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct HookContext {
    pub worktree_name: String,
    pub worktree_path: PathBuf,
    pub branch: String,
    pub base_branch: Option<String>,
    pub base_path: PathBuf,
}

pub struct HookRunner {
    rsworktree_dir: PathBuf,
}

impl HookRunner {
    pub fn new(rsworktree_dir: &Path) -> Self {
        Self {
            rsworktree_dir: rsworktree_dir.to_path_buf(),
        }
    }

    pub fn hooks_dir(&self) -> PathBuf {
        self.rsworktree_dir.join(HOOKS_DIR)
    }

    pub fn hook_path(&self, hook: HookName) -> PathBuf {
        self.hooks_dir().join(hook.as_str())
    }

    pub fn run_hook(&self, hook: HookName, context: &HookContext) -> color_eyre::Result<()> {
        let hook_path = self.hook_path(hook);

        if !hook_path.exists() {
            return Ok(());
        }

        if !is_executable(&hook_path) {
            let path_display = hook_path.display();
            let hint = format!(
                "{}",
                "hint: make the hook executable with `chmod +x`"
                    .if_supports_color(Stream::Stderr, |text| format!("{}", text.dimmed()))
            );
            eprintln!(
                "Warning: hook `{}` exists but is not executable.\n{hint}",
                path_display
            );
            return Ok(());
        }

        let hook_name = format!(
            "{}",
            hook.as_str()
                .if_supports_color(Stream::Stdout, |text| format!("{}", text.cyan()))
        );
        println!("Running {} hook...", hook_name);

        let status = Command::new(&hook_path)
            .current_dir(&context.worktree_path)
            .env("RSWORKTREE_NAME", &context.worktree_name)
            .env("RSWORKTREE_PATH", &context.worktree_path)
            .env("RSWORKTREE_BRANCH", &context.branch)
            .env(
                "RSWORKTREE_BASE_BRANCH",
                context.base_branch.as_deref().unwrap_or(""),
            )
            .env("RSWORKTREE_BASE_PATH", &context.base_path)
            .status()
            .wrap_err_with(|| {
                eyre::eyre!("failed to execute hook `{}`", hook_path.display())
            })?;

        if !status.success() {
            let code = status.code().unwrap_or(-1);
            let warning = format!(
                "{}",
                format!("Warning: hook `{}` exited with code {code}", hook.as_str())
                    .if_supports_color(Stream::Stderr, |text| format!("{}", text.yellow()))
            );
            eprintln!("{warning}");
        }

        Ok(())
    }
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn hook_name_as_str() {
        assert_eq!(HookName::PostCreate.as_str(), "post-create");
    }

    #[test]
    fn hook_path_is_correct() {
        let dir = TempDir::new().unwrap();
        let runner = HookRunner::new(dir.path());

        let expected = dir.path().join("hooks").join("post-create");
        assert_eq!(runner.hook_path(HookName::PostCreate), expected);
    }

    #[test]
    fn run_hook_does_nothing_when_hook_missing() -> color_eyre::Result<()> {
        let dir = TempDir::new()?;
        let runner = HookRunner::new(dir.path());

        let context = HookContext {
            worktree_name: "test".into(),
            worktree_path: dir.path().to_path_buf(),
            branch: "feature/test".into(),
            base_branch: Some("main".into()),
            base_path: dir.path().to_path_buf(),
        };

        // Should not error when hook doesn't exist
        runner.run_hook(HookName::PostCreate, &context)?;

        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn run_hook_executes_script() -> color_eyre::Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let dir = TempDir::new()?;
        let hooks_dir = dir.path().join("hooks");
        fs::create_dir_all(&hooks_dir)?;

        let hook_path = hooks_dir.join("post-create");
        let marker_file = dir.path().join("hook_ran");

        // Create a simple script that creates a marker file
        fs::write(
            &hook_path,
            format!(
                "#!/bin/sh\necho \"$RSWORKTREE_NAME\" > {:?}\n",
                marker_file
            ),
        )?;

        // Make it executable
        let mut perms = fs::metadata(&hook_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&hook_path, perms)?;

        let runner = HookRunner::new(dir.path());
        let context = HookContext {
            worktree_name: "my-worktree".into(),
            worktree_path: dir.path().to_path_buf(),
            branch: "feature/test".into(),
            base_branch: None,
            base_path: dir.path().to_path_buf(),
        };

        runner.run_hook(HookName::PostCreate, &context)?;

        assert!(marker_file.exists(), "hook should have created marker file");
        let content = fs::read_to_string(&marker_file)?;
        assert_eq!(content.trim(), "my-worktree");

        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn run_hook_warns_when_not_executable() -> color_eyre::Result<()> {
        let dir = TempDir::new()?;
        let hooks_dir = dir.path().join("hooks");
        fs::create_dir_all(&hooks_dir)?;

        let hook_path = hooks_dir.join("post-create");
        fs::write(&hook_path, "#!/bin/sh\necho test\n")?;

        // Do NOT make it executable

        let runner = HookRunner::new(dir.path());
        let context = HookContext {
            worktree_name: "test".into(),
            worktree_path: dir.path().to_path_buf(),
            branch: "feature/test".into(),
            base_branch: None,
            base_path: dir.path().to_path_buf(),
        };

        // Should not error, just warn
        runner.run_hook(HookName::PostCreate, &context)?;

        Ok(())
    }
}
