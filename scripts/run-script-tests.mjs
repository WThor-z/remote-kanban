#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { constants } from "node:fs";
import { access, readdir } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const findScriptTests = async (repoRoot = process.cwd()) => {
  const testsDir = path.join(repoRoot, "scripts", "__tests__");

  try {
    await access(testsDir, constants.F_OK);
  } catch {
    return [];
  }

  const entries = await readdir(testsDir, { withFileTypes: true });
  return entries
    .filter((entry) => entry.isFile() && entry.name.endsWith(".test.mjs"))
    .map((entry) => path.join(testsDir, entry.name))
    .sort((a, b) => a.localeCompare(b));
};

const runScriptTests = async () => {
  const files = await findScriptTests();
  if (files.length === 0) {
    console.error("No script tests found under scripts/__tests__.");
    process.exit(1);
  }

  const result = spawnSync(process.execPath, ["--test", ...files], {
    stdio: "inherit",
  });

  if (result.error) {
    console.error(`Failed to launch node --test: ${result.error.message}`);
    process.exit(1);
  }

  process.exit(result.status ?? 1);
};

const entryPath = fileURLToPath(import.meta.url);
if (process.argv[1] && path.resolve(process.argv[1]) === entryPath) {
  runScriptTests().catch((error) => {
    console.error(`Script test runner failed: ${error instanceof Error ? error.message : String(error)}`);
    process.exit(1);
  });
}
