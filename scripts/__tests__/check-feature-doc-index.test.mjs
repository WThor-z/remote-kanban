import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";

const SCRIPT_PATH = path.resolve(process.cwd(), "scripts/check-feature-doc-index.mjs");

const runScript = (cwd) =>
  spawnSync(process.execPath, [SCRIPT_PATH], {
    cwd,
    encoding: "utf8",
  });

const createRepo = async () => {
  const repoDir = await mkdtemp(path.join(tmpdir(), "feature-doc-index-"));
  const featuresDir = path.join(repoDir, "docs", "features");
  await mkdir(featuresDir, { recursive: true });
  return { repoDir, featuresDir };
};

test("passes when every feature doc is registered in index", async (t) => {
  const { repoDir, featuresDir } = await createRepo();
  t.after(async () => rm(repoDir, { recursive: true, force: true }));

  await writeFile(
    path.join(featuresDir, "index.md"),
    "# Feature Catalog\n\n- [Task Commands](task-commands.md)\n",
    "utf8",
  );
  await writeFile(path.join(featuresDir, "task-commands.md"), "# Task Commands\n", "utf8");
  await writeFile(path.join(featuresDir, "_template.md"), "# Template\n", "utf8");

  const result = runScript(repoDir);
  assert.equal(result.status, 0, result.stderr || result.stdout);
  assert.match(result.stdout, /passed/i);
});

test("fails when feature docs exist but are missing from index", async (t) => {
  const { repoDir, featuresDir } = await createRepo();
  t.after(async () => rm(repoDir, { recursive: true, force: true }));

  await writeFile(path.join(featuresDir, "index.md"), "# Feature Catalog\n", "utf8");
  await writeFile(path.join(featuresDir, "task-commands.md"), "# Task Commands\n", "utf8");

  const result = runScript(repoDir);
  assert.equal(result.status, 1, "expected non-zero exit for missing index entries");
  assert.match(result.stderr, /Missing from docs\/features\/index\.md:/);
  assert.match(result.stderr, /task-commands\.md/);
});

test("fails when index references non-existent feature docs", async (t) => {
  const { repoDir, featuresDir } = await createRepo();
  t.after(async () => rm(repoDir, { recursive: true, force: true }));

  await writeFile(
    path.join(featuresDir, "index.md"),
    "# Feature Catalog\n\n- [Ghost](ghost.md)\n",
    "utf8",
  );

  const result = runScript(repoDir);
  assert.equal(result.status, 1, "expected non-zero exit for stale index references");
  assert.match(result.stderr, /file not found/i);
  assert.match(result.stderr, /ghost\.md/);
});
