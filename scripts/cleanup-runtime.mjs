#!/usr/bin/env node

import { access, rm } from "node:fs/promises";
import { constants } from "node:fs";
import { execFileSync } from "node:child_process";
import path from "node:path";

const args = new Set(process.argv.slice(2));
const apply = args.has("--apply");
const runsOnly = args.has("--runs-only");
const worktreesOnly = args.has("--worktrees-only");

if (runsOnly && worktreesOnly) {
  console.error("Use only one of --runs-only or --worktrees-only.");
  process.exit(1);
}

const targets = runsOnly ? ["runs"] : worktreesOnly ? ["worktrees"] : ["runs", "worktrees"];
const modeLabel = apply ? "APPLY" : "DRY-RUN";
const repoRoot = process.cwd();
const dataRoots = [path.join(repoRoot, ".vk-data"), path.join(repoRoot, "crates", ".vk-data")];

const exists = async (targetPath) => {
  try {
    await access(targetPath, constants.F_OK);
    return true;
  } catch {
    return false;
  }
};

const candidates = [];
for (const root of dataRoots) {
  for (const target of targets) {
    const targetPath = path.join(root, target);
    if (await exists(targetPath)) {
      candidates.push(targetPath);
    }
  }
}

console.log(`[${modeLabel}] runtime cleanup`);
if (candidates.length === 0) {
  console.log("No runtime directories found for cleanup.");
  process.exit(0);
}

for (const candidate of candidates) {
  const rel = path.relative(repoRoot, candidate) || candidate;
  if (apply) {
    await rm(candidate, { recursive: true, force: true });
    console.log(`Removed: ${rel}`);
  } else {
    console.log(`Would remove: ${rel}`);
  }
}

if (apply && targets.includes("worktrees")) {
  try {
    execFileSync("git", ["worktree", "prune", "--expire", "now", "--verbose"], {
      stdio: "inherit",
    });
  } catch (error) {
    console.warn(`Warning: git worktree prune failed (${error instanceof Error ? error.message : String(error)})`);
  }
}

if (!apply) {
  console.log("Use --apply to perform deletion.");
}
