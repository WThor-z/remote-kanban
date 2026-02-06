import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { access, mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { constants } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

const SCRIPT_PATH = path.resolve(process.cwd(), "scripts/cleanup-runtime.mjs");

const exists = async (targetPath) => {
  try {
    await access(targetPath, constants.F_OK);
    return true;
  } catch {
    return false;
  }
};

const runScript = (cwd, args = []) =>
  spawnSync(process.execPath, [SCRIPT_PATH, ...args], {
    cwd,
    encoding: "utf8",
  });

const createRuntimeTree = async () => {
  const repoDir = await mkdtemp(path.join(tmpdir(), "cleanup-runtime-"));
  const rootRuns = path.join(repoDir, ".vk-data", "runs", "task-a");
  const rootWorktrees = path.join(repoDir, ".vk-data", "worktrees", "wt-a");
  const cratesRuns = path.join(repoDir, "crates", ".vk-data", "runs", "task-b");
  const cratesWorktrees = path.join(repoDir, "crates", ".vk-data", "worktrees", "wt-b");

  await mkdir(rootRuns, { recursive: true });
  await mkdir(rootWorktrees, { recursive: true });
  await mkdir(cratesRuns, { recursive: true });
  await mkdir(cratesWorktrees, { recursive: true });

  await writeFile(path.join(rootRuns, "run.json"), "{}", "utf8");
  await writeFile(path.join(rootWorktrees, "marker.txt"), "x", "utf8");
  await writeFile(path.join(cratesRuns, "run.json"), "{}", "utf8");
  await writeFile(path.join(cratesWorktrees, "marker.txt"), "x", "utf8");

  return {
    repoDir,
    rootRuns: path.join(repoDir, ".vk-data", "runs"),
    rootWorktrees: path.join(repoDir, ".vk-data", "worktrees"),
    cratesRuns: path.join(repoDir, "crates", ".vk-data", "runs"),
    cratesWorktrees: path.join(repoDir, "crates", ".vk-data", "worktrees"),
  };
};

test("dry-run lists cleanup targets without deleting data", async (t) => {
  const runtime = await createRuntimeTree();
  t.after(async () => rm(runtime.repoDir, { recursive: true, force: true }));

  const result = runScript(runtime.repoDir);
  assert.equal(result.status, 0, result.stderr || result.stdout);
  assert.match(result.stdout, /\[DRY-RUN\] runtime cleanup/);
  assert.match(result.stdout, /Would remove:/);

  assert.equal(await exists(runtime.rootRuns), true);
  assert.equal(await exists(runtime.rootWorktrees), true);
  assert.equal(await exists(runtime.cratesRuns), true);
  assert.equal(await exists(runtime.cratesWorktrees), true);
});

test("apply with --runs-only removes runs directories and keeps worktrees", async (t) => {
  const runtime = await createRuntimeTree();
  t.after(async () => rm(runtime.repoDir, { recursive: true, force: true }));

  const result = runScript(runtime.repoDir, ["--apply", "--runs-only"]);
  assert.equal(result.status, 0, result.stderr || result.stdout);

  assert.equal(await exists(runtime.rootRuns), false);
  assert.equal(await exists(runtime.cratesRuns), false);
  assert.equal(await exists(runtime.rootWorktrees), true);
  assert.equal(await exists(runtime.cratesWorktrees), true);
});

test("conflicting selector flags fail fast", async (t) => {
  const runtime = await createRuntimeTree();
  t.after(async () => rm(runtime.repoDir, { recursive: true, force: true }));

  const result = runScript(runtime.repoDir, ["--runs-only", "--worktrees-only"]);
  assert.equal(result.status, 1);
  assert.match(result.stderr, /Use only one of --runs-only or --worktrees-only/);
});
