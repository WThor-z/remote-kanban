#!/usr/bin/env node

import { spawn } from "node:child_process";
import path from "node:path";

import { loadEnvFile, resolveGatewayEnv } from "./gateway-launcher.mjs";

const printUsage = () => {
  console.error("Usage: node scripts/run-gateway.mjs <local|cloud> [--start]");
};

const mode = process.argv[2];
const runMode = process.argv.includes("--start") ? "start" : "dev";

if (!mode || (mode !== "local" && mode !== "cloud")) {
  printUsage();
  process.exit(1);
}

const repoRoot = process.cwd();
const envFilePath = path.join(repoRoot, ".env.gateway");
const envFileValues = mode === "cloud" ? await loadEnvFile(envFilePath) : {};

let resolvedEnv;
try {
  resolvedEnv = resolveGatewayEnv({
    mode,
    env: process.env,
    envFileValues,
  });
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  if (mode === "cloud") {
    console.error("Create .env.gateway with GATEWAY_SERVER_URL and GATEWAY_AUTH_TOKEN.");
  }
  process.exit(1);
}

const command = process.platform === "win32" ? "pnpm.cmd" : "pnpm";
const args = ["--dir", "services/agent-gateway", runMode];

const child = spawn(command, args, {
  stdio: "inherit",
  env: resolvedEnv,
  shell: process.platform === "win32",
});

child.on("error", (error) => {
  console.error(`Failed to start gateway: ${error instanceof Error ? error.message : String(error)}`);
  process.exit(1);
});

child.on("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }
  process.exit(code ?? 0);
});
