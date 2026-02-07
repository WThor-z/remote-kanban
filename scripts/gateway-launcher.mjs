import { access, readFile } from "node:fs/promises";
import { constants } from "node:fs";

const LOCAL_DEFAULTS = {
  GATEWAY_SERVER_URL: "ws://127.0.0.1:8081",
  GATEWAY_AUTH_TOKEN: "dev-token",
};

const REQUIRED_CLOUD_KEYS = ["GATEWAY_SERVER_URL", "GATEWAY_AUTH_TOKEN"];

const stripQuotes = (value) => {
  if (
    (value.startsWith('"') && value.endsWith('"')) ||
    (value.startsWith("'") && value.endsWith("'"))
  ) {
    return value.slice(1, -1);
  }
  return value;
};

export const parseDotEnv = (content) => {
  const values = {};

  for (const rawLine of content.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line || line.startsWith("#")) {
      continue;
    }

    const separatorIndex = line.indexOf("=");
    if (separatorIndex <= 0) {
      continue;
    }

    const key = line.slice(0, separatorIndex).trim();
    if (!key) {
      continue;
    }

    const value = stripQuotes(line.slice(separatorIndex + 1).trim());
    values[key] = value;
  }

  return values;
};

const mergeEnv = (base, additions) => {
  const merged = { ...base };
  for (const [key, value] of Object.entries(additions)) {
    if (!merged[key] && value) {
      merged[key] = value;
    }
  }
  return merged;
};

export const resolveGatewayEnv = ({ mode, env, envFileValues = {} }) => {
  if (mode !== "local" && mode !== "cloud") {
    throw new Error(`Unsupported gateway mode: ${mode}`);
  }

  const baseEnv = { ...(env || {}) };

  if (mode === "local") {
    return mergeEnv(baseEnv, LOCAL_DEFAULTS);
  }

  const merged = mergeEnv(baseEnv, envFileValues);

  const missing = REQUIRED_CLOUD_KEYS.filter((key) => !(merged[key] || "").trim());
  if (missing.length > 0) {
    throw new Error(
      `Missing required gateway config for cloud mode: ${missing.join(", ")}. Set them in environment variables or .env.gateway.`,
    );
  }

  return merged;
};

export const loadEnvFile = async (filePath) => {
  try {
    await access(filePath, constants.F_OK);
  } catch {
    return {};
  }

  const content = await readFile(filePath, "utf8");
  return parseDotEnv(content);
};
