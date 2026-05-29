use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use crate::executor::Executor;

fn worktree_base() -> Result<PathBuf> {
    let home = dirs::home_dir().context("failed to resolve home directory")?;
    Ok(home.join(".claude-mux").join("worktrees"))
}

fn repo_name(repo: &Path) -> Result<String> {
    repo.file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .context("failed to extract repo name")
}

pub fn worktree_path(repo: &Path, branch: &str) -> Result<PathBuf> {
    let name = repo_name(repo)?;
    let sanitized = branch.replace('/', "-");
    let base = worktree_base()?;
    Ok(base.join(&name).join(format!("{name}-{sanitized}")))
}

// NOTE: 現在はExecutor(外部コマンド)経由でgit CLIを呼んでいるが、
// git2等のライブラリに切り替える場合はWorktree自体をtrait化し、
// CLI実装(現在)とライブラリ実装を差し替え可能にする設計が考えられる。
pub struct Worktree<E: Executor> {
    executor: E,
}

impl<E: Executor> Worktree<E> {
    pub fn new(executor: E) -> Self {
        Self { executor }
    }

    pub fn resolve_branch(&self, repo: &Path) -> Result<String> {
        let output = self.executor.execute(
            "git",
            &["rev-parse", "--abbrev-ref", "HEAD"],
            Some(repo),
        )?;
        Ok(output.stdout)
    }

    pub fn is_git_repo(&self, path: &Path) -> bool {
        self.executor
            .execute("git", &["rev-parse", "--git-dir"], Some(path))
            .is_ok()
    }

    pub fn create(&self, repo: &Path, branch: &str) -> Result<PathBuf> {
        let wt_path = worktree_path(repo, branch)?;

        if wt_path.exists() {
            bail!("worktree already exists: {}", wt_path.display());
        }

        let repo = std::fs::canonicalize(repo)
            .with_context(|| format!("failed to canonicalize {}", repo.display()))?;

        self.executor.execute(
            "git",
            &["worktree", "add", &wt_path.to_string_lossy(), branch],
            Some(&repo),
        )?;

        Ok(wt_path)
    }

    pub fn remove(&self, repo: &Path, branch: &str) -> Result<()> {
        let wt_path = worktree_path(repo, branch)?;

        if !wt_path.exists() {
            return Ok(());
        }

        if self.is_dirty(&wt_path)? {
            bail!("worktree has uncommitted changes: {}", wt_path.display());
        }

        let repo = std::fs::canonicalize(repo)
            .with_context(|| format!("failed to canonicalize {}", repo.display()))?;

        self.executor.execute(
            "git",
            &["worktree", "remove", &wt_path.to_string_lossy()],
            Some(&repo),
        )?;

        Ok(())
    }

    pub fn force_remove(&self, repo: &Path, branch: &str) -> Result<()> {
        let wt_path = worktree_path(repo, branch)?;

        if !wt_path.exists() {
            return Ok(());
        }

        let repo = std::fs::canonicalize(repo)
            .with_context(|| format!("failed to canonicalize {}", repo.display()))?;

        self.executor.execute(
            "git",
            &["worktree", "remove", "--force", &wt_path.to_string_lossy()],
            Some(&repo),
        )?;

        Ok(())
    }

    fn is_dirty(&self, wt_path: &Path) -> Result<bool> {
        let output = self.executor.execute(
            "git",
            &["status", "--porcelain"],
            Some(wt_path),
        )?;
        Ok(!output.stdout.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command as StdCommand;

    use crate::executor::CommandExecutor;


    fn init_test_repo(dir: &Path) {
        StdCommand::new("git").args(["init"]).current_dir(dir).output().unwrap();
        StdCommand::new("git").args(["checkout", "-b", "main"]).current_dir(dir).output().unwrap();
        fs::write(dir.join("README.md"), "test").unwrap();
        StdCommand::new("git").args(["add", "."]).current_dir(dir).output().unwrap();
        StdCommand::new("git").args(["commit", "-m", "init"]).current_dir(dir).output().unwrap();
    }

    #[test]
    fn test_worktree_path() {
        let path = worktree_path(Path::new("/home/user/myrepo"), "feature/auth").unwrap();
        let home = dirs::home_dir().unwrap();
        assert_eq!(
            path,
            home.join(".claude-mux/worktrees/myrepo/myrepo-feature-auth")
        );
    }

    #[test]
    fn test_is_git_repo() {
        let dir = tempfile::tempdir().unwrap();
        let wt = Worktree::new(CommandExecutor);
        assert!(!wt.is_git_repo(dir.path()));
        init_test_repo(dir.path());
        assert!(wt.is_git_repo(dir.path()));
    }

    #[test]
    fn test_resolve_branch() {
        let dir = tempfile::tempdir().unwrap();
        init_test_repo(dir.path());
        let wt = Worktree::new(CommandExecutor);
        let branch = wt.resolve_branch(dir.path()).unwrap();
        assert_eq!(branch, "main");
    }

    #[test]
    fn test_create_and_remove() {
        let dir = tempfile::tempdir().unwrap();
        init_test_repo(dir.path());

        StdCommand::new("git")
            .args(["branch", "feature-test"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        let wt = Worktree::new(CommandExecutor);
        let wt_path = wt.create(dir.path(), "feature-test").unwrap();
        assert!(wt_path.exists());

        wt.remove(dir.path(), "feature-test").unwrap();
        assert!(!wt_path.exists());
    }

    #[test]
    fn test_create_duplicate_errors() {
        let dir = tempfile::tempdir().unwrap();
        init_test_repo(dir.path());

        StdCommand::new("git")
            .args(["branch", "dup-branch"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        let wt = Worktree::new(CommandExecutor);
        wt.create(dir.path(), "dup-branch").unwrap();
        let err = wt.create(dir.path(), "dup-branch").unwrap_err();
        assert!(err.to_string().contains("worktree already exists"));

        wt.force_remove(dir.path(), "dup-branch").unwrap();
    }

    #[test]
    fn test_remove_dirty_worktree_errors() {
        let dir = tempfile::tempdir().unwrap();
        init_test_repo(dir.path());

        StdCommand::new("git")
            .args(["branch", "dirty-branch"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        let wt = Worktree::new(CommandExecutor);
        let wt_path = wt.create(dir.path(), "dirty-branch").unwrap();
        fs::write(wt_path.join("dirty.txt"), "uncommitted").unwrap();

        let err = wt.remove(dir.path(), "dirty-branch").unwrap_err();
        assert!(err.to_string().contains("uncommitted changes"));

        wt.force_remove(dir.path(), "dirty-branch").unwrap();
    }
}
