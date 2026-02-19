//! Git command execution utilities

#![allow(dead_code)] // Some functions are for future use

use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, trace};

use crate::error::{Result, WorktreeError};

/// Output from a git command
#[derive(Debug)]
pub struct GitOutput {
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

/// Execute a git command in the specified directory
pub async fn git_command(repo_path: &Path, args: &[&str]) -> Result<GitOutput> {
    debug!("Running git {:?} in {:?}", args, repo_path);

    let output = Command::new("git")
        .args(args)
        .current_dir(repo_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| WorktreeError::git_failed_with_source("Failed to execute git command", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    trace!("git stdout: {}", stdout);
    if !stderr.is_empty() {
        trace!("git stderr: {}", stderr);
    }

    Ok(GitOutput {
        stdout,
        stderr,
        success: output.status.success(),
    })
}

/// Execute a git command and return error if it fails
pub async fn git_command_checked(repo_path: &Path, args: &[&str]) -> Result<String> {
    let output = git_command(repo_path, args).await?;

    if !output.success {
        return Err(WorktreeError::git_failed(format!(
            "git {} failed: {}",
            args.join(" "),
            output.stderr.trim()
        )));
    }

    Ok(output.stdout)
}

/// Check if a path is inside a git repository
pub async fn is_git_repository(path: &Path) -> Result<bool> {
    let output = git_command(path, &["rev-parse", "--git-dir"]).await?;
    Ok(output.success)
}

/// Get the root directory of the git repository
pub async fn get_repo_root(path: &Path) -> Result<std::path::PathBuf> {
    let output = git_command_checked(path, &["rev-parse", "--show-toplevel"]).await?;
    Ok(std::path::PathBuf::from(output.trim()))
}

/// Check if a branch exists
pub async fn branch_exists(repo_path: &Path, branch: &str) -> Result<bool> {
    let output = git_command(
        repo_path,
        &["rev-parse", "--verify", &format!("refs/heads/{}", branch)],
    )
    .await?;
    Ok(output.success)
}

/// Check if a remote branch exists
pub async fn remote_branch_exists(repo_path: &Path, remote: &str, branch: &str) -> Result<bool> {
    let output = git_command(
        repo_path,
        &[
            "rev-parse",
            "--verify",
            &format!("refs/remotes/{}/{}", remote, branch),
        ],
    )
    .await?;
    Ok(output.success)
}

/// Get the current commit hash for a branch
pub async fn get_branch_commit(repo_path: &Path, branch: &str) -> Result<String> {
    let output = git_command_checked(repo_path, &["rev-parse", branch]).await?;
    Ok(output.trim().to_string())
}

/// Create a new branch from a base
pub async fn create_branch(repo_path: &Path, branch: &str, base: &str) -> Result<()> {
    git_command_checked(repo_path, &["branch", branch, base]).await?;
    Ok(())
}

/// Delete a branch
pub async fn delete_branch(repo_path: &Path, branch: &str, force: bool) -> Result<()> {
    let flag = if force { "-D" } else { "-d" };
    git_command_checked(repo_path, &["branch", flag, branch]).await?;
    Ok(())
}

/// Fetch from remote
pub async fn fetch(repo_path: &Path, remote: &str) -> Result<()> {
    git_command_checked(repo_path, &["fetch", remote]).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn init_test_repo() -> TempDir {
        let dir = TempDir::new().unwrap();
        git_command_checked(dir.path(), &["init"]).await.unwrap();
        git_command_checked(dir.path(), &["config", "user.email", "test@test.com"])
            .await
            .unwrap();
        git_command_checked(dir.path(), &["config", "user.name", "Test"])
            .await
            .unwrap();

        // Create initial commit
        let test_file = dir.path().join("test.txt");
        tokio::fs::write(&test_file, "test").await.unwrap();
        git_command_checked(dir.path(), &["add", "."])
            .await
            .unwrap();
        git_command_checked(dir.path(), &["commit", "-m", "Initial commit"])
            .await
            .unwrap();

        dir
    }

    #[tokio::test]
    async fn test_is_git_repository() {
        let dir = init_test_repo().await;
        assert!(is_git_repository(dir.path()).await.unwrap());

        let non_git = TempDir::new().unwrap();
        assert!(!is_git_repository(non_git.path()).await.unwrap());
    }

    #[tokio::test]
    async fn test_branch_exists() {
        let dir = init_test_repo().await;

        // Default branch should exist (master or main)
        let has_master = branch_exists(dir.path(), "master").await.unwrap();
        let has_main = branch_exists(dir.path(), "main").await.unwrap();
        assert!(has_master || has_main);

        // Non-existent branch
        assert!(!branch_exists(dir.path(), "nonexistent").await.unwrap());
    }

    #[tokio::test]
    async fn test_create_and_delete_branch() {
        let dir = init_test_repo().await;

        // Get the default branch name
        let default_branch = if branch_exists(dir.path(), "master").await.unwrap() {
            "master"
        } else {
            "main"
        };

        // Create branch
        create_branch(dir.path(), "test-branch", default_branch)
            .await
            .unwrap();
        assert!(branch_exists(dir.path(), "test-branch").await.unwrap());

        // Delete branch
        delete_branch(dir.path(), "test-branch", false)
            .await
            .unwrap();
        assert!(!branch_exists(dir.path(), "test-branch").await.unwrap());
    }
}
