use std::fmt;

use serde::{Deserialize, Serialize};

/// Git hosting provider for merge/pull request operations.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GitProvider {
    #[default]
    GitHub,
    GitLab,
}

impl GitProvider {
    /// Returns the CLI program name for this provider.
    pub fn cli_program(&self) -> &'static str {
        match self {
            GitProvider::GitHub => "gh",
            GitProvider::GitLab => "glab",
        }
    }

    /// Returns a human-readable display name for the provider.
    pub fn display_name(&self) -> &'static str {
        match self {
            GitProvider::GitHub => "GitHub",
            GitProvider::GitLab => "GitLab",
        }
    }

    /// Returns the term for merge/pull requests on this provider.
    pub fn merge_request_term(&self) -> &'static str {
        match self {
            GitProvider::GitHub => "pull request",
            GitProvider::GitLab => "merge request",
        }
    }

    /// Returns the short term for merge/pull requests (PR or MR).
    pub fn merge_request_short(&self) -> &'static str {
        match self {
            GitProvider::GitHub => "PR",
            GitProvider::GitLab => "MR",
        }
    }

    /// Build arguments for creating a pull/merge request.
    pub fn build_create_args(
        &self,
        branch: &str,
        draft: bool,
        fill: bool,
        web: bool,
        reviewers: &[String],
        extra_args: &[String],
    ) -> Vec<String> {
        let mut args = match self {
            GitProvider::GitHub => vec!["pr".to_owned(), "create".to_owned()],
            GitProvider::GitLab => vec!["mr".to_owned(), "create".to_owned()],
        };

        // Branch specification differs between providers
        match self {
            GitProvider::GitHub => {
                args.push("--head".to_owned());
                args.push(branch.to_owned());
            }
            GitProvider::GitLab => {
                args.push("--source-branch".to_owned());
                args.push(branch.to_owned());
            }
        }

        if draft {
            args.push("--draft".to_owned());
        }

        if fill {
            match self {
                GitProvider::GitHub => args.push("--fill".to_owned()),
                GitProvider::GitLab => args.push("--fill".to_owned()),
            }
        }

        if web {
            match self {
                GitProvider::GitHub => args.push("--web".to_owned()),
                GitProvider::GitLab => args.push("--web".to_owned()),
            }
        }

        for reviewer in reviewers {
            match self {
                GitProvider::GitHub => {
                    args.push("--reviewer".to_owned());
                    args.push(reviewer.clone());
                }
                GitProvider::GitLab => {
                    args.push("--reviewer".to_owned());
                    args.push(reviewer.clone());
                }
            }
        }

        args.extend(extra_args.iter().cloned());

        args
    }

    /// Build arguments for listing open pull/merge requests.
    pub fn build_list_args(&self, branch: &str) -> Vec<String> {
        match self {
            GitProvider::GitHub => vec![
                "pr".to_owned(),
                "list".to_owned(),
                "--head".to_owned(),
                branch.to_owned(),
                "--state".to_owned(),
                "open".to_owned(),
                "--json".to_owned(),
                "number".to_owned(),
                "--limit".to_owned(),
                "1".to_owned(),
            ],
            GitProvider::GitLab => vec![
                "mr".to_owned(),
                "list".to_owned(),
                "--source-branch".to_owned(),
                branch.to_owned(),
                "--state".to_owned(),
                "opened".to_owned(),
                "--output".to_owned(),
                "json".to_owned(),
            ],
        }
    }

    /// Build arguments for merging a pull/merge request.
    pub fn build_merge_args(&self, mr_number: u64, delete_branch: bool) -> Vec<String> {
        match self {
            GitProvider::GitHub => {
                let mut args = vec![
                    "pr".to_owned(),
                    "merge".to_owned(),
                    mr_number.to_string(),
                    "--merge".to_owned(),
                ];
                if delete_branch {
                    args.push("--delete-branch".to_owned());
                }
                args
            }
            GitProvider::GitLab => {
                let mut args = vec![
                    "mr".to_owned(),
                    "merge".to_owned(),
                    mr_number.to_string(),
                ];
                if delete_branch {
                    args.push("--remove-source-branch".to_owned());
                }
                args
            }
        }
    }

    /// Check if the command output indicates a branch delete failure.
    pub fn is_branch_delete_failure(&self, stderr: &str) -> bool {
        let stderr_lower = stderr.to_lowercase();
        match self {
            GitProvider::GitHub => {
                stderr_lower.contains("failed to delete local branch")
                    || stderr_lower.contains("cannot delete branch")
            }
            GitProvider::GitLab => {
                stderr_lower.contains("failed to delete")
                    || stderr_lower.contains("could not remove")
            }
        }
    }
}

impl fmt::Display for GitProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl std::str::FromStr for GitProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "github" | "gh" => Ok(GitProvider::GitHub),
            "gitlab" | "glab" => Ok(GitProvider::GitLab),
            _ => Err(format!(
                "unknown provider '{}', expected 'github' or 'gitlab'",
                s
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_provider_is_github() {
        assert_eq!(GitProvider::default(), GitProvider::GitHub);
    }

    #[test]
    fn cli_program_returns_correct_binary() {
        assert_eq!(GitProvider::GitHub.cli_program(), "gh");
        assert_eq!(GitProvider::GitLab.cli_program(), "glab");
    }

    #[test]
    fn display_name_returns_human_readable() {
        assert_eq!(GitProvider::GitHub.display_name(), "GitHub");
        assert_eq!(GitProvider::GitLab.display_name(), "GitLab");
    }

    #[test]
    fn merge_request_term_differs_by_provider() {
        assert_eq!(GitProvider::GitHub.merge_request_term(), "pull request");
        assert_eq!(GitProvider::GitLab.merge_request_term(), "merge request");
    }

    #[test]
    fn merge_request_short_differs_by_provider() {
        assert_eq!(GitProvider::GitHub.merge_request_short(), "PR");
        assert_eq!(GitProvider::GitLab.merge_request_short(), "MR");
    }

    #[test]
    fn build_create_args_github_basic() {
        let args = GitProvider::GitHub.build_create_args(
            "feature/test",
            false,
            false,
            false,
            &[],
            &[],
        );
        assert_eq!(
            args,
            vec!["pr", "create", "--head", "feature/test"]
        );
    }

    #[test]
    fn build_create_args_gitlab_basic() {
        let args = GitProvider::GitLab.build_create_args(
            "feature/test",
            false,
            false,
            false,
            &[],
            &[],
        );
        assert_eq!(
            args,
            vec!["mr", "create", "--source-branch", "feature/test"]
        );
    }

    #[test]
    fn build_create_args_with_all_options() {
        let reviewers = vec!["alice".to_owned(), "bob".to_owned()];
        let extra = vec!["--label".to_owned(), "bug".to_owned()];

        let github_args = GitProvider::GitHub.build_create_args(
            "feature/test",
            true,
            true,
            true,
            &reviewers,
            &extra,
        );
        assert!(github_args.contains(&"--draft".to_owned()));
        assert!(github_args.contains(&"--fill".to_owned()));
        assert!(github_args.contains(&"--web".to_owned()));
        assert!(github_args.contains(&"--reviewer".to_owned()));
        assert!(github_args.contains(&"alice".to_owned()));
        assert!(github_args.contains(&"bob".to_owned()));
        assert!(github_args.contains(&"--label".to_owned()));
        assert!(github_args.contains(&"bug".to_owned()));

        let gitlab_args = GitProvider::GitLab.build_create_args(
            "feature/test",
            true,
            true,
            true,
            &reviewers,
            &extra,
        );
        assert!(gitlab_args.contains(&"--draft".to_owned()));
        assert!(gitlab_args.contains(&"--fill".to_owned()));
        assert!(gitlab_args.contains(&"--web".to_owned()));
    }

    #[test]
    fn build_list_args_github() {
        let args = GitProvider::GitHub.build_list_args("feature/test");
        assert!(args.contains(&"pr".to_owned()));
        assert!(args.contains(&"list".to_owned()));
        assert!(args.contains(&"--head".to_owned()));
        assert!(args.contains(&"--state".to_owned()));
        assert!(args.contains(&"open".to_owned()));
    }

    #[test]
    fn build_list_args_gitlab() {
        let args = GitProvider::GitLab.build_list_args("feature/test");
        assert!(args.contains(&"mr".to_owned()));
        assert!(args.contains(&"list".to_owned()));
        assert!(args.contains(&"--source-branch".to_owned()));
        assert!(args.contains(&"--state".to_owned()));
        assert!(args.contains(&"opened".to_owned()));
    }

    #[test]
    fn build_merge_args_github() {
        let args = GitProvider::GitHub.build_merge_args(42, true);
        assert_eq!(
            args,
            vec!["pr", "merge", "42", "--merge", "--delete-branch"]
        );

        let args_no_delete = GitProvider::GitHub.build_merge_args(42, false);
        assert_eq!(args_no_delete, vec!["pr", "merge", "42", "--merge"]);
    }

    #[test]
    fn build_merge_args_gitlab() {
        let args = GitProvider::GitLab.build_merge_args(42, true);
        assert_eq!(
            args,
            vec!["mr", "merge", "42", "--remove-source-branch"]
        );

        let args_no_delete = GitProvider::GitLab.build_merge_args(42, false);
        assert_eq!(args_no_delete, vec!["mr", "merge", "42"]);
    }

    #[test]
    fn is_branch_delete_failure_github() {
        assert!(GitProvider::GitHub.is_branch_delete_failure("failed to delete local branch"));
        assert!(GitProvider::GitHub.is_branch_delete_failure("cannot delete branch"));
        assert!(!GitProvider::GitHub.is_branch_delete_failure("success"));
    }

    #[test]
    fn is_branch_delete_failure_gitlab() {
        assert!(GitProvider::GitLab.is_branch_delete_failure("failed to delete"));
        assert!(GitProvider::GitLab.is_branch_delete_failure("could not remove"));
        assert!(!GitProvider::GitLab.is_branch_delete_failure("success"));
    }

    #[test]
    fn from_str_parses_valid_providers() {
        assert_eq!("github".parse::<GitProvider>().unwrap(), GitProvider::GitHub);
        assert_eq!("GitHub".parse::<GitProvider>().unwrap(), GitProvider::GitHub);
        assert_eq!("gh".parse::<GitProvider>().unwrap(), GitProvider::GitHub);
        assert_eq!("gitlab".parse::<GitProvider>().unwrap(), GitProvider::GitLab);
        assert_eq!("GitLab".parse::<GitProvider>().unwrap(), GitProvider::GitLab);
        assert_eq!("glab".parse::<GitProvider>().unwrap(), GitProvider::GitLab);
    }

    #[test]
    fn from_str_errors_on_unknown() {
        assert!("unknown".parse::<GitProvider>().is_err());
    }

    #[test]
    fn display_shows_provider_name() {
        assert_eq!(format!("{}", GitProvider::GitHub), "GitHub");
        assert_eq!(format!("{}", GitProvider::GitLab), "GitLab");
    }

    #[test]
    fn serde_roundtrip() {
        let github = GitProvider::GitHub;
        let json = serde_json::to_string(&github).unwrap();
        assert_eq!(json, "\"github\"");
        let parsed: GitProvider = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, GitProvider::GitHub);

        let gitlab = GitProvider::GitLab;
        let json = serde_json::to_string(&gitlab).unwrap();
        assert_eq!(json, "\"gitlab\"");
        let parsed: GitProvider = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, GitProvider::GitLab);
    }
}
