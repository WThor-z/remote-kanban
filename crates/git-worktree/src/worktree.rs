//! Worktree management

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::commands::{
    branch_exists, delete_branch, git_command, git_command_checked, is_git_repository,
};
use crate::error::{Result, WorktreeError};

/// Status of a worktree
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorktreeStatus {
    /// Worktree is active and usable
    Active,
    /// Worktree is locked
    Locked,
    /// Worktree is prunable (orphaned)
    Prunable,
}

/// Represents a Git worktree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Worktree {
    /// Absolute path to the worktree
    pub path: PathBuf,
    /// Branch name associated with this worktree
    pub branch: String,
    /// Commit hash at the HEAD of this worktree
    pub head: String,
    /// Status of the worktree
    pub status: WorktreeStatus,
    /// Whether this is the main worktree
    pub is_main: bool,
}

/// Configuration for WorktreeManager
#[derive(Debug, Clone)]
pub struct WorktreeConfig {
    /// Directory where worktrees will be created
    pub worktree_dir: PathBuf,
    /// Prefix for worktree branch names
    pub branch_prefix: String,
}

impl Default for WorktreeConfig {
    fn default() -> Self {
        Self {
            worktree_dir: PathBuf::from(".worktrees"),
            branch_prefix: "task/".to_string(),
        }
    }
}

/// Manages Git worktrees for task isolation
#[derive(Debug)]
pub struct WorktreeManager {
    /// Path to the main repository
    repo_path: PathBuf,
    /// Configuration
    config: WorktreeConfig,
}

impl WorktreeManager {
    /// Create a new WorktreeManager for the given repository
    pub async fn new(repo_path: impl Into<PathBuf>) -> Result<Self> {
        let repo_path = repo_path.into();

        // Verify it's a git repository
        if !is_git_repository(&repo_path).await? {
            return Err(WorktreeError::NotAGitRepository {
                path: repo_path.clone(),
            });
        }

        Ok(Self {
            repo_path,
            config: WorktreeConfig::default(),
        })
    }

    /// Create a new WorktreeManager with custom configuration
    pub async fn with_config(
        repo_path: impl Into<PathBuf>,
        config: WorktreeConfig,
    ) -> Result<Self> {
        let repo_path = repo_path.into();

        if !is_git_repository(&repo_path).await? {
            return Err(WorktreeError::NotAGitRepository {
                path: repo_path.clone(),
            });
        }

        Ok(Self { repo_path, config })
    }

    /// Get the repository path
    pub fn repo_path(&self) -> &Path {
        &self.repo_path
    }

    /// Get the worktree directory
    pub fn worktree_dir(&self) -> PathBuf {
        self.repo_path.join(&self.config.worktree_dir)
    }

    /// Create a new worktree for a task
    ///
    /// # Arguments
    /// * `task_id` - Unique identifier for the task
    /// * `base_branch` - The branch to base the new worktree on (e.g., "main")
    ///
    /// # Returns
    /// The created Worktree
    pub async fn create(&self, task_id: &str, base_branch: &str) -> Result<Worktree> {
        // Generate branch name
        let branch_name = format!("{}{}", self.config.branch_prefix, task_id);

        // Check if branch already exists
        if branch_exists(&self.repo_path, &branch_name).await? {
            return Err(WorktreeError::BranchExists {
                branch: branch_name,
            });
        }

        // Check if base branch exists
        if !branch_exists(&self.repo_path, base_branch).await? {
            return Err(WorktreeError::BranchNotFound {
                branch: base_branch.to_string(),
            });
        }

        // Create worktree directory if it doesn't exist
        let worktree_dir = self.worktree_dir();
        tokio::fs::create_dir_all(&worktree_dir).await?;

        // Generate worktree path
        let worktree_path = worktree_dir.join(task_id);

        // Check if worktree path already exists
        if worktree_path.exists() {
            return Err(WorktreeError::WorktreeExists {
                path: worktree_path,
            });
        }

        info!(
            "Creating worktree at {:?} from branch {}",
            worktree_path, base_branch
        );

        // Create the worktree with a new branch
        git_command_checked(
            &self.repo_path,
            &[
                "worktree",
                "add",
                "-b",
                &branch_name,
                worktree_path.to_str().unwrap(),
                base_branch,
            ],
        )
        .await?;

        // Get the HEAD commit
        let head = git_command_checked(&worktree_path, &["rev-parse", "HEAD"]).await?;

        Ok(Worktree {
            path: worktree_path,
            branch: branch_name,
            head: head.trim().to_string(),
            status: WorktreeStatus::Active,
            is_main: false,
        })
    }

    /// Create a worktree with an auto-generated task ID
    pub async fn create_auto(&self, base_branch: &str) -> Result<Worktree> {
        let task_id = Uuid::new_v4().to_string();
        self.create(&task_id, base_branch).await
    }

    /// List all worktrees
    pub async fn list(&self) -> Result<Vec<Worktree>> {
        let output =
            git_command_checked(&self.repo_path, &["worktree", "list", "--porcelain"]).await?;

        let mut worktrees = Vec::new();
        let mut current_worktree: Option<Worktree> = None;

        for line in output.lines() {
            if line.starts_with("worktree ") {
                // Save previous worktree if exists
                if let Some(wt) = current_worktree.take() {
                    worktrees.push(wt);
                }

                let path = PathBuf::from(line.trim_start_matches("worktree "));
                current_worktree = Some(Worktree {
                    path,
                    branch: String::new(),
                    head: String::new(),
                    status: WorktreeStatus::Active,
                    is_main: false,
                });
            } else if let Some(ref mut wt) = current_worktree {
                if line.starts_with("HEAD ") {
                    wt.head = line.trim_start_matches("HEAD ").to_string();
                } else if line.starts_with("branch ") {
                    let branch = line.trim_start_matches("branch refs/heads/");
                    wt.branch = branch.to_string();
                } else if line == "bare" {
                    wt.is_main = true;
                } else if line == "locked" {
                    wt.status = WorktreeStatus::Locked;
                } else if line == "prunable" {
                    wt.status = WorktreeStatus::Prunable;
                }
            }
        }

        // Don't forget the last worktree
        if let Some(wt) = current_worktree {
            worktrees.push(wt);
        }

        // Mark the first worktree as main (it's the primary working directory)
        if let Some(first) = worktrees.first_mut() {
            first.is_main = true;
        }

        Ok(worktrees)
    }

    /// Get a specific worktree by path
    pub async fn get(&self, path: &Path) -> Result<Option<Worktree>> {
        let worktrees = self.list().await?;
        Ok(worktrees.into_iter().find(|wt| wt.path == path))
    }

    /// Get a worktree by task ID
    pub async fn get_by_task_id(&self, task_id: &str) -> Result<Option<Worktree>> {
        let worktree_path = self.worktree_dir().join(task_id);
        self.get(&worktree_path).await
    }

    /// Remove a worktree
    ///
    /// # Arguments
    /// * `path` - Path to the worktree to remove
    /// * `force` - Force removal even if there are uncommitted changes
    /// * `delete_branch` - Also delete the associated branch
    pub async fn remove(&self, path: &Path, force: bool, delete_branch_flag: bool) -> Result<()> {
        // Get worktree info first
        let worktree = self
            .get(path)
            .await?
            .ok_or_else(|| WorktreeError::WorktreeNotFound {
                path: path.to_path_buf(),
            })?;

        if worktree.is_main {
            return Err(WorktreeError::git_failed("Cannot remove the main worktree"));
        }

        info!("Removing worktree at {:?}", path);

        // Remove the worktree
        let mut args = vec!["worktree", "remove"];
        if force {
            args.push("--force");
        }
        args.push(path.to_str().unwrap());

        git_command_checked(&self.repo_path, &args).await?;

        // Optionally delete the branch
        if delete_branch_flag && !worktree.branch.is_empty() {
            debug!("Deleting branch {}", worktree.branch);
            if let Err(e) = delete_branch(&self.repo_path, &worktree.branch, force).await {
                warn!("Failed to delete branch {}: {}", worktree.branch, e);
            }
        }

        Ok(())
    }

    /// Remove a worktree by task ID
    pub async fn remove_by_task_id(
        &self,
        task_id: &str,
        force: bool,
        delete_branch: bool,
    ) -> Result<()> {
        let worktree_path = self.worktree_dir().join(task_id);
        self.remove(&worktree_path, force, delete_branch).await
    }

    /// Prune stale worktree information
    pub async fn prune(&self) -> Result<()> {
        git_command_checked(&self.repo_path, &["worktree", "prune"]).await?;
        Ok(())
    }

    /// Clean up orphaned worktrees (prunable ones)
    ///
    /// # Returns
    /// Number of worktrees cleaned up
    pub async fn cleanup_orphans(&self) -> Result<usize> {
        let worktrees = self.list().await?;
        let mut cleaned = 0;

        for wt in worktrees {
            if wt.status == WorktreeStatus::Prunable {
                info!("Cleaning up orphaned worktree at {:?}", wt.path);
                if let Err(e) = self.remove(&wt.path, true, true).await {
                    warn!("Failed to remove orphaned worktree: {}", e);
                } else {
                    cleaned += 1;
                }
            }
        }

        // Also run git worktree prune
        self.prune().await?;

        Ok(cleaned)
    }

    /// Lock a worktree to prevent accidental removal
    pub async fn lock(&self, path: &Path, reason: Option<&str>) -> Result<()> {
        let mut args = vec!["worktree", "lock", path.to_str().unwrap()];
        if let Some(r) = reason {
            args.push("--reason");
            args.push(r);
        }
        git_command_checked(&self.repo_path, &args).await?;
        Ok(())
    }

    /// Unlock a worktree
    pub async fn unlock(&self, path: &Path) -> Result<()> {
        git_command_checked(
            &self.repo_path,
            &["worktree", "unlock", path.to_str().unwrap()],
        )
        .await?;
        Ok(())
    }

    /// Check if there are uncommitted changes in a worktree
    pub async fn has_uncommitted_changes(&self, worktree_path: &Path) -> Result<bool> {
        let output = git_command(worktree_path, &["status", "--porcelain"]).await?;
        Ok(!output.stdout.trim().is_empty())
    }

    /// Get the diff of changes in a worktree
    pub async fn get_diff(&self, worktree_path: &Path) -> Result<String> {
        let output = git_command_checked(worktree_path, &["diff", "HEAD"]).await?;
        Ok(output)
    }

    /// Commit all changes in a worktree
    pub async fn commit_all(&self, worktree_path: &Path, message: &str) -> Result<String> {
        // Stage all changes
        git_command_checked(worktree_path, &["add", "-A"]).await?;

        // Commit
        git_command_checked(worktree_path, &["commit", "-m", message]).await?;

        // Get the new commit hash
        let output = git_command_checked(worktree_path, &["rev-parse", "HEAD"]).await?;
        Ok(output.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::git_command_checked;
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

        // Create initial commit on main branch
        git_command_checked(dir.path(), &["checkout", "-b", "main"])
            .await
            .ok(); // Ignore error if already on main

        let test_file = dir.path().join("test.txt");
        tokio::fs::write(&test_file, "test content").await.unwrap();
        git_command_checked(dir.path(), &["add", "."])
            .await
            .unwrap();
        git_command_checked(dir.path(), &["commit", "-m", "Initial commit"])
            .await
            .unwrap();

        dir
    }

    #[tokio::test]
    async fn test_create_worktree_manager() {
        let dir = init_test_repo().await;
        let manager = WorktreeManager::new(dir.path()).await.unwrap();
        assert_eq!(manager.repo_path(), dir.path());
    }

    #[tokio::test]
    async fn test_create_worktree_manager_not_git_repo() {
        let dir = TempDir::new().unwrap();
        let result = WorktreeManager::new(dir.path()).await;
        assert!(matches!(
            result,
            Err(WorktreeError::NotAGitRepository { .. })
        ));
    }

    #[tokio::test]
    async fn test_create_and_list_worktree() {
        let dir = init_test_repo().await;
        let manager = WorktreeManager::new(dir.path()).await.unwrap();

        // Create a worktree
        let worktree = manager.create("test-task", "main").await.unwrap();
        assert!(worktree.path.exists());
        assert_eq!(worktree.branch, "task/test-task");
        assert_eq!(worktree.status, WorktreeStatus::Active);
        assert!(!worktree.is_main);

        // List worktrees
        let worktrees = manager.list().await.unwrap();
        assert!(worktrees.len() >= 2); // Main + our new one

        // Find our worktree
        let found = worktrees.iter().find(|wt| wt.branch == "task/test-task");
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_create_duplicate_worktree() {
        let dir = init_test_repo().await;
        let manager = WorktreeManager::new(dir.path()).await.unwrap();

        // Create first worktree
        manager.create("test-task", "main").await.unwrap();

        // Try to create duplicate
        let result = manager.create("test-task", "main").await;
        assert!(matches!(result, Err(WorktreeError::BranchExists { .. })));
    }

    #[tokio::test]
    async fn test_remove_worktree() {
        let dir = init_test_repo().await;
        let manager = WorktreeManager::new(dir.path()).await.unwrap();

        // Create a worktree
        let worktree = manager.create("test-task", "main").await.unwrap();
        let path = worktree.path.clone();
        assert!(path.exists());

        // Remove it
        manager.remove(&path, false, true).await.unwrap();
        assert!(!path.exists());

        // Verify branch is also deleted
        assert!(!branch_exists(dir.path(), "task/test-task").await.unwrap());
    }

    #[tokio::test]
    async fn test_get_by_task_id() {
        let dir = init_test_repo().await;
        let manager = WorktreeManager::new(dir.path()).await.unwrap();

        // Create a worktree
        manager.create("my-task", "main").await.unwrap();

        // Get by task ID
        let found = manager.get_by_task_id("my-task").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().branch, "task/my-task");

        // Non-existent task
        let not_found = manager.get_by_task_id("nonexistent").await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_has_uncommitted_changes() {
        let dir = init_test_repo().await;
        let manager = WorktreeManager::new(dir.path()).await.unwrap();

        // Create a worktree
        let worktree = manager.create("test-task", "main").await.unwrap();

        // Initially no changes
        assert!(!manager
            .has_uncommitted_changes(&worktree.path)
            .await
            .unwrap());

        // Make a change
        let new_file = worktree.path.join("new_file.txt");
        tokio::fs::write(&new_file, "new content").await.unwrap();

        // Now should have changes
        assert!(manager
            .has_uncommitted_changes(&worktree.path)
            .await
            .unwrap());
    }
}
