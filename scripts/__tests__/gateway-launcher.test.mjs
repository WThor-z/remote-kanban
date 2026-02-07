import { test } from "node:test";
import assert from "node:assert/strict";

import { parseDotEnv, resolveGatewayEnv } from "../gateway-launcher.mjs";

test("parseDotEnv parses key/value pairs and ignores comments", () => {
  const parsed = parseDotEnv(
    [
      "# comment",
      "GATEWAY_SERVER_URL=wss://kanban.example.com",
      "GATEWAY_AUTH_TOKEN=\"secret-token\"",
      "GATEWAY_HOST_ID=worker-01",
      "",
    ].join("\n"),
  );

  assert.deepEqual(parsed, {
    GATEWAY_SERVER_URL: "wss://kanban.example.com",
    GATEWAY_AUTH_TOKEN: "secret-token",
    GATEWAY_HOST_ID: "worker-01",
  });
});

test("resolveGatewayEnv uses local defaults", () => {
  const env = resolveGatewayEnv({ mode: "local", env: {} });

  assert.equal(env.GATEWAY_SERVER_URL, "ws://127.0.0.1:8081");
  assert.equal(env.GATEWAY_AUTH_TOKEN, "dev-token");
});

test("resolveGatewayEnv cloud mode uses env-file values", () => {
  const env = resolveGatewayEnv({
    mode: "cloud",
    env: {},
    envFileValues: {
      GATEWAY_SERVER_URL: "wss://kanban.example.com",
      GATEWAY_AUTH_TOKEN: "cloud-token",
    },
  });

  assert.equal(env.GATEWAY_SERVER_URL, "wss://kanban.example.com");
  assert.equal(env.GATEWAY_AUTH_TOKEN, "cloud-token");
});

test("resolveGatewayEnv cloud mode throws when required values are missing", () => {
  assert.throws(
    () => resolveGatewayEnv({ mode: "cloud", env: {}, envFileValues: {} }),
    /Missing required gateway config/i,
  );
});
