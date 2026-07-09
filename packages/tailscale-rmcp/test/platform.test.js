"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const {
  downloadUrl,
  releaseBaseUrl,
  releaseVersion,
  targetFor,
} = require("../lib/platform");
const { version: packageVersion } = require("../package.json");

test("maps supported platforms to release assets", () => {
  assert.deepEqual(targetFor("linux", "x64"), {
    asset: "rtailscale-x86_64.tar.gz",
    binary: "rtailscale",
  });
  assert.deepEqual(targetFor("win32", "x64"), {
    asset: "rtailscale-windows-x86_64.tar.gz",
    binary: "rtailscale.exe",
  });
});

test("rejects unsupported platforms", () => {
  assert.throws(() => targetFor("darwin", "arm64"), /Unsupported platform/);
});

test("uses npm package version as the binary tag by default", () => {
  assert.equal(releaseVersion({}), `v${packageVersion}`);
});

test("allows release tag and repo overrides", () => {
  const env = { TAILSCALE_RMCP_BINARY_VERSION: "v9.9.9", TAILSCALE_RMCP_REPO: "example/tailscale-rmcp" };
  assert.equal(releaseBaseUrl(env), "https://github.com/example/tailscale-rmcp/releases/download");
  assert.equal(downloadUrl(targetFor("linux", "x64"), env), "https://github.com/example/tailscale-rmcp/releases/download/v9.9.9/rtailscale-x86_64.tar.gz");
});
