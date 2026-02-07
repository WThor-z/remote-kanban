import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";

import { findScriptTests } from "../run-script-tests.mjs";

test("findScriptTests returns sorted *.test.mjs files only", async (t) => {
  const repoDir = await mkdtemp(path.join(tmpdir(), "script-tests-"));
  t.after(async () => rm(repoDir, { recursive: true, force: true }));

  const testsDir = path.join(repoDir, "scripts", "__tests__");
  await mkdir(testsDir, { recursive: true });

  const alpha = path.join(testsDir, "alpha.test.mjs");
  const zulu = path.join(testsDir, "zulu.test.mjs");

  await writeFile(path.join(testsDir, "README.md"), "ignore", "utf8");
  await writeFile(path.join(testsDir, "helper.mjs"), "ignore", "utf8");
  await writeFile(zulu, "", "utf8");
  await writeFile(alpha, "", "utf8");

  const found = await findScriptTests(repoDir);
  assert.deepEqual(found, [alpha, zulu]);
});

test("findScriptTests returns empty list when tests dir is missing", async (t) => {
  const repoDir = await mkdtemp(path.join(tmpdir(), "script-tests-missing-"));
  t.after(async () => rm(repoDir, { recursive: true, force: true }));

  const found = await findScriptTests(repoDir);
  assert.deepEqual(found, []);
});
