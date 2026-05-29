use std::path::Path;
use std::process::Command as StdCommand;

use anyhow::{bail, Context, Result};


pub struct CommandOutput {
    pub stdout: String,
}

// NOTE: このtraitのシグネチャは外部コマンド実行に特化している。
// git2等のライブラリに切り替える場合、Executor経由では対応できないため、
// Worktree自体をtrait化してCLI実装とライブラリ実装を差し替える設計が必要になる。
// 現状はテスト時のモック差し替えとボイラープレート削減が主な役割。
pub trait Executor {
    fn execute(&self, program: &str, args: &[&str], cwd: Option<&Path>) -> Result<CommandOutput>;
}

impl<T: Executor> Executor for &T {
    fn execute(&self, program: &str, args: &[&str], cwd: Option<&Path>) -> Result<CommandOutput> {
        (**self).execute(program, args, cwd)
    }
}

pub struct CommandExecutor;

impl Executor for CommandExecutor {
    fn execute(&self, program: &str, args: &[&str], cwd: Option<&Path>) -> Result<CommandOutput> {
        let mut cmd = StdCommand::new(program);
        cmd.args(args);
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }

        let output = cmd.output()
            .with_context(|| format!("failed to run {program}"))?;

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("{program} failed: {}", stderr.trim());
        }

        Ok(CommandOutput { stdout })
    }
}

#[cfg(test)]
pub struct MockExecutor {
    responses: std::cell::RefCell<Vec<Result<CommandOutput>>>,
}

#[cfg(test)]
impl MockExecutor {
    pub fn new(responses: Vec<Result<CommandOutput>>) -> Self {
        let mut responses = responses;
        responses.reverse();
        Self {
            responses: std::cell::RefCell::new(responses),
        }
    }

    pub fn ok(stdout: &str) -> Result<CommandOutput> {
        Ok(CommandOutput {
            stdout: stdout.to_string(),
        })
    }

    pub fn err(message: &str) -> Result<CommandOutput> {
        Err(anyhow::anyhow!("{message}"))
    }
}

#[cfg(test)]
impl Executor for MockExecutor {
    fn execute(&self, _program: &str, _args: &[&str], _cwd: Option<&Path>) -> Result<CommandOutput> {
        self.responses
            .borrow_mut()
            .pop()
            .unwrap_or_else(|| Err(anyhow::anyhow!("no more mock responses")))
    }
}
