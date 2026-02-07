/**
 * Git Operations Module
 *
 * Provides Git worktree management for task isolation.
 * All Git operations run on the Gateway host.
 */

import { execFile } from 'child_process';
import { promisify } from 'util';
import * as fs from 'fs';
import * as path from 'path';

const execFileAsync = promisify(execFile);

export interface WorktreeInfo {
  path: string;
  branch: string;
  head: string;
  isMain: boolean;
}

export interface GitOperationResult {
  success: boolean;
  message?: string;
  commitHash?: string;
  error?: string;
}

/**
 * Git operations manager for a project
 */
export class GitOperations {
  constructor(
    private projectPath: string,
    private worktreeDir: string = '.worktrees',
    private branchPrefix: string = 'task/'
  ) {}

  /**
   * Run a git command in a specific directory
   */
  private async git(args: string[], cwd?: string): Promise<string> {
    const workDir = cwd || this.projectPath;
    const { stdout } = await execFileAsync('git', args, { cwd: workDir });
    return stdout.trim();
  }

  private formatGitError(err: unknown): string {
    if (err instanceof Error) {
      const typed = err as Error & { stdout?: string; stderr?: string };
      const output = [typed.stderr, typed.stdout, err.message]
        .filter((value): value is string => Boolean(value && value.trim()))
        .join('\n');
      return output.trim() || err.message;
    }

    return String(err);
  }

  private isMergeConflict(err: unknown): boolean {
    const message = this.formatGitError(err);
    return message.includes('CONFLICT') || message.includes('Automatic merge failed');
  }

  /**
   * Check if a branch exists
   */
  async branchExists(branch: string): Promise<boolean> {
    try {
      await this.git(['rev-parse', '--verify', branch]);
      return true;
    } catch {
      return false;
    }
  }

  /**
   * Get the worktrees directory path
   */
  getWorktreesPath(): string {
    return path.join(this.projectPath, this.worktreeDir);
  }

  /**
   * Create a worktree for a task
   */
  async createWorktree(
    taskId: string,
    baseBranch: string = 'main'
  ): Promise<WorktreeInfo> {
    const branchName = `${this.branchPrefix}${taskId}`;
    const worktreePath = path.join(this.getWorktreesPath(), taskId);

    // Check if branch already exists
    if (await this.branchExists(branchName)) {
      throw new Error(`Branch ${branchName} already exists`);
    }

    // Create worktrees directory if needed
    const wtDir = this.getWorktreesPath();
    if (!fs.existsSync(wtDir)) {
      fs.mkdirSync(wtDir, { recursive: true });
    }

    // Create the worktree
    await this.git(['worktree', 'add', '-b', branchName, worktreePath, baseBranch]);

    // Get HEAD commit
    const head = await this.git(['rev-parse', 'HEAD'], worktreePath);

    return {
      path: worktreePath,
      branch: branchName,
      head,
      isMain: false,
    };
  }

  /**
   * Check if a worktree exists for a task
   */
  async worktreeExists(taskId: string): Promise<boolean> {
    const worktreePath = path.join(this.getWorktreesPath(), taskId);
    return fs.existsSync(worktreePath);
  }

  /**
   * Get worktree path for a task
   */
  getWorktreePath(taskId: string): string {
    return path.join(this.getWorktreesPath(), taskId);
  }

  /**
   * Check for uncommitted changes in a worktree
   */
  async hasUncommittedChanges(worktreePath: string): Promise<boolean> {
    const status = await this.git(['status', '--porcelain'], worktreePath);
    return status.length > 0;
  }

  /**
   * Stage and commit all changes
   */
  async commitAll(
    worktreePath: string,
    message: string
  ): Promise<GitOperationResult> {
    try {
      // Stage all changes
      await this.git(['add', '-A'], worktreePath);

      // Check if there are staged changes
      const status = await this.git(['status', '--porcelain'], worktreePath);
      if (!status) {
        return { success: true, message: 'No changes to commit' };
      }

      // Commit
      await this.git(['commit', '-m', message], worktreePath);

      // Get commit hash
      const commitHash = await this.git(['rev-parse', 'HEAD'], worktreePath);

      return { success: true, commitHash, message: `Committed ${commitHash}` };
    } catch (err) {
      return {
        success: false,
        error: err instanceof Error ? err.message : String(err),
      };
    }
  }

  /**
   * Merge a task branch to the default branch
   */
  async mergeToMain(
    taskBranch: string,
    targetBranch: string = 'main',
    conflictStrategy: 'ai_resolve' | 'keep_branch' = 'keep_branch'
  ): Promise<GitOperationResult> {
    try {
      // First, checkout the target branch in main repo
      await this.git(['checkout', targetBranch]);

      // Pull latest changes
      try {
        await this.git(['pull', '--rebase']);
      } catch {
        // Pull might fail if no remote, continue anyway
      }

      // Merge the task branch
      await this.git(['merge', '--no-ff', taskBranch, '-m', `Merge ${taskBranch}`]);

      const commitHash = await this.git(['rev-parse', 'HEAD']);

      return { success: true, commitHash, message: `Merged ${taskBranch} to ${targetBranch}` };
    } catch (err) {
      const conflictDetected = this.isMergeConflict(err);

      if (conflictDetected && conflictStrategy === 'ai_resolve') {
        try {
          await this.git(['merge', '--abort']);
        } catch {
          // Ignore abort errors
        }

        try {
          await this.git([
            'merge',
            '--no-ff',
            '-X',
            'theirs',
            taskBranch,
            '-m',
            `Merge ${taskBranch}`,
          ]);
          const commitHash = await this.git(['rev-parse', 'HEAD']);
          return {
            success: true,
            commitHash,
            message: `Merged ${taskBranch} with auto-resolve`,
          };
        } catch (autoErr) {
          if (this.isMergeConflict(autoErr)) {
            try {
              await this.git(['merge', '--abort']);
            } catch {
              // Ignore abort errors
            }
            return { success: false, error: 'Merge conflict detected' };
          }

          return { success: false, error: this.formatGitError(autoErr) };
        }
      }

      if (conflictDetected) {
        try {
          await this.git(['merge', '--abort']);
        } catch {
          // Ignore abort errors
        }
        return { success: false, error: 'Merge conflict detected' };
      }

      return { success: false, error: this.formatGitError(err) };
    }
  }

  /**
   * Push changes to remote
   */
  async push(branch?: string): Promise<GitOperationResult> {
    try {
      const args = ['push'];
      if (branch) {
        args.push('origin', branch);
      }
      await this.git(args);
      return { success: true, message: `Pushed ${branch || 'current branch'}` };
    } catch (err) {
      return {
        success: false,
        error: err instanceof Error ? err.message : String(err),
      };
    }
  }

  /**
   * Push with set-upstream for new branches
   */
  async pushNewBranch(branch: string): Promise<GitOperationResult> {
    try {
      await this.git(['push', '-u', 'origin', branch]);
      return { success: true, message: `Pushed ${branch} with tracking` };
    } catch (err) {
      return {
        success: false,
        error: err instanceof Error ? err.message : String(err),
      };
    }
  }

  /**
   * Remove a worktree
   */
  async removeWorktree(
    taskId: string,
    force: boolean = false,
    deleteBranch: boolean = false
  ): Promise<GitOperationResult> {
    try {
      const worktreePath = this.getWorktreePath(taskId);
      const branchName = `${this.branchPrefix}${taskId}`;

      // Remove worktree
      const args = ['worktree', 'remove'];
      if (force) {
        args.push('--force');
      }
      args.push(worktreePath);
      await this.git(args);

      // Optionally delete branch
      if (deleteBranch) {
        try {
          const deleteArgs = ['branch', '-d'];
          if (force) {
            deleteArgs[1] = '-D';
          }
          deleteArgs.push(branchName);
          await this.git(deleteArgs);
        } catch {
          // Branch deletion might fail, continue
        }
      }

      return { success: true, message: `Removed worktree for ${taskId}` };
    } catch (err) {
      return {
        success: false,
        error: err instanceof Error ? err.message : String(err),
      };
    }
  }

  /**
   * Get the current branch name
   */
  async getCurrentBranch(worktreePath?: string): Promise<string> {
    return this.git(['branch', '--show-current'], worktreePath);
  }

  /**
   * Get diff summary
   */
  async getDiffStat(worktreePath: string): Promise<string> {
    try {
      return await this.git(['diff', '--stat', 'HEAD'], worktreePath);
    } catch {
      return '';
    }
  }
}
