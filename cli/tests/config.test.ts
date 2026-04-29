// Tests for config loading and resolution.

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import path from "path";
import os from "os";
import fs from "fs";

import {
  loadConfig,
  resolveRpcUrl,
  resolveMint,
  type RtpConfig,
} from "../src/config.js";

describe("config defaults", () => {
  it("loads with defaults when no config files exist", () => {
    // The test just ensures loadConfig doesn't throw
    const config = loadConfig();
    assert.equal(config.cluster, "devnet");
    assert.equal(config.rpcUrl, null);
    assert.equal(config.defaultMint, null);
    assert.ok(config.feePayerKeypairPath.includes("solana"));
  });
});

describe("resolveRpcUrl", () => {
  it("returns devnet RPC for devnet cluster", () => {
    const rpc = resolveRpcUrl({ cluster: "devnet" } as RtpConfig);
    assert.equal(rpc, "https://api.devnet.solana.com");
  });

  it("returns mainnet RPC for mainnet cluster", () => {
    const rpc = resolveRpcUrl({ cluster: "mainnet" } as RtpConfig);
    assert.equal(rpc, "https://api.mainnet-beta.solana.com");
  });

  it("prefers config rpcUrl over cluster default", () => {
    const rpc = resolveRpcUrl({ cluster: "devnet", rpcUrl: "https://custom.rpc" } as RtpConfig);
    assert.equal(rpc, "https://custom.rpc");
  });
});

describe("resolveMint", () => {
  it("returns flag mint when provided", () => {
    const mint = resolveMint("FlagMint123", { defaultMint: "ConfigMint456" } as RtpConfig);
    assert.equal(mint, "FlagMint123");
  });

  it("returns config defaultMint when no flag", () => {
    const mint = resolveMint(undefined, { defaultMint: "ConfigMint456" } as RtpConfig);
    assert.equal(mint, "ConfigMint456");
  });

  it("throws when neither flag nor config mint", () => {
    assert.throws(
      () => resolveMint(undefined, { defaultMint: null } as RtpConfig),
      /No mint specified/,
    );
  });
});
