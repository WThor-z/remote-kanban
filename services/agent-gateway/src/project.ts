/**
 * Project Detection Module
 *
 * Detects Git repository information from the current working directory.
 * This information is sent to the central server during Gateway registration.
 */

import { exec } from 'child_process';
import { promisify } from 'util';
import * as fs from 'fs';
import * as path from 'path';

const execAsync = promisify(exec);

export interface ProjectInfo {
  /** Project name (from directory name or --project-name) */
  name: string;
  /** Absolute path to the project root */
  localPath: string;
  /** Remote Git URL (origin) */
  remoteUrl: string | null;
  /** Default branch name */
  defaultBranch: string;
  /** Whether this is a valid Git repository */
  isGitRepo: boolean;
}

/**
 * Detect project information from a directory
 */
export async function detectProject(
  projectPath: string,
  projectName?: string
): Promise<ProjectInfo> {
  const absolutePath = path.resolve(projectPath);

  // Check if .git exists
  const gitDir = path.join(absolutePath, '.git');
  const isGitRepo = fs.existsSync(gitDir);

  if (!isGitRepo) {
    return {
      name: projectName || path.basename(absolutePath),
      localPath: absolutePath,
      remoteUrl: null,
      defaultBranch: 'main',
      isGitRepo: false,
    };
  }

  // Get project name from directory if not provided
  const name = projectName || path.basename(absolutePath);

  // Get remote URL
  const remoteUrl = await getRemoteUrl(absolutePath);

  // Get default branch
  const defaultBranch = await getDefaultBranch(absolutePath);

  return {
    name,
    localPath: absolutePath,
    remoteUrl,
    defaultBranch,
    isGitRepo: true,
  };
}

/**
 * Get the remote URL (origin)
 */
async function getRemoteUrl(cwd: string): Promise<string | null> {
  try {
    const { stdout } = await execAsync('git remote get-url origin', { cwd });
    return stdout.trim() || null;
  } catch {
    // No remote configured
    return null;
  }
}

/**
 * Get the default branch name
 */
async function getDefaultBranch(cwd: string): Promise<string> {
  try {
    // Try to get from remote HEAD reference
    const { stdout } = await execAsync(
      'git symbolic-ref refs/remotes/origin/HEAD',
      { cwd }
    );
    // Output is like "refs/remotes/origin/main"
    const parts = stdout.trim().split('/');
    return parts[parts.length - 1] || 'main';
  } catch {
    try {
      // Fallback: get current branch
      const { stdout } = await execAsync('git branch --show-current', { cwd });
      return stdout.trim() || 'main';
    } catch {
      return 'main';
    }
  }
}

/**
 * Validate that a path is a Git repository
 */
export function validateGitRepo(projectPath: string): void {
  const absolutePath = path.resolve(projectPath);
  const gitDir = path.join(absolutePath, '.git');

  if (!fs.existsSync(absolutePath)) {
    throw new Error(`Project path does not exist: ${absolutePath}`);
  }

  if (!fs.existsSync(gitDir)) {
    throw new Error(
      `Not a git repository: ${absolutePath}\n` +
        'The Gateway must be started in a Git repository directory.'
    );
  }
}
