#!/usr/bin/env node

import { readdir, readFile } from "node:fs/promises";
import path from "node:path";

const repoRoot = process.cwd();
const featuresDir = path.join(repoRoot, "docs", "features");
const indexPath = path.join(featuresDir, "index.md");
const ignoredFiles = new Set(["index.md", "_template.md"]);

const fail = (message) => {
  console.error(message);
  process.exit(1);
};

const toSet = (values) => new Set(values);

try {
  const files = await readdir(featuresDir);
  const featureDocs = files
    .filter((name) => name.endsWith(".md") && !ignoredFiles.has(name))
    .sort();

  const indexContent = await readFile(indexPath, "utf8");
  const linkMatches = [...indexContent.matchAll(/\[[^\]]+\]\(([^)]+\.md)\)/g)];
  const indexedDocs = linkMatches
    .map((match) => path.basename(match[1]))
    .filter((name) => !ignoredFiles.has(name));

  const featureSet = toSet(featureDocs);
  const indexedSet = toSet(indexedDocs);

  const missingInIndex = featureDocs.filter((name) => !indexedSet.has(name));
  const staleInIndex = [...indexedSet].filter((name) => !featureSet.has(name)).sort();

  if (missingInIndex.length === 0 && staleInIndex.length === 0) {
    console.log(`Feature docs index check passed (${featureDocs.length} docs registered).`);
    process.exit(0);
  }

  if (missingInIndex.length > 0) {
    console.error("Missing from docs/features/index.md:");
    for (const name of missingInIndex) {
      console.error(`- ${name}`);
    }
  }

  if (staleInIndex.length > 0) {
    console.error("Referenced in docs/features/index.md but file not found:");
    for (const name of staleInIndex) {
      console.error(`- ${name}`);
    }
  }

  process.exit(1);
} catch (error) {
  fail(`Feature docs index check failed: ${error instanceof Error ? error.message : String(error)}`);
}
