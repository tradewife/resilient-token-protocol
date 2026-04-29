// Tests for keypair loading and formatting.

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import fs from "fs";
import path from "path";
import os from "os";

import { Keypair } from "@solana/web3.js";

import {
  loadKeypair,
  truncatePubkey,
  formatSol,
  formatSolFull,
  isHotWallet,
} from "../src/keypair.js";

describe("truncatePubkey", () => {
  it("truncates long base58 strings", () => {
    const kp = Keypair.generate();
    const truncated = truncatePubkey(kp.publicKey);
    assert.ok(truncated.length < kp.publicKey.toBase58().length);
    assert.ok(truncated.includes("..."));
    assert.equal(truncated.length, 6 + 3 + 4); // first6...last4
  });

  it("returns short strings unchanged", () => {
    assert.equal(truncatePubkey("abc"), "abc");
  });

  it("accepts string input", () => {
    const truncated = truncatePubkey("11111111111111111111111111111111");
    assert.ok(truncated.includes("..."));
  });
});

describe("formatSol", () => {
  it("formats lamports to 4 decimal places", () => {
    assert.equal(formatSol(1_500_000_000), "1.5000");
  });

  it("formats zero", () => {
    assert.equal(formatSol(0), "0.0000");
  });

  it("formats small amounts", () => {
    assert.equal(formatSol(123_456_789), "0.1235");
  });
});

describe("formatSolFull", () => {
  it("formats lamports to 9 decimal places", () => {
    assert.equal(formatSolFull(1_500_000_000), "1.500000000");
  });
});

describe("isHotWallet", () => {
  it("detects solana config path as hot wallet", () => {
    assert.ok(isHotWallet(path.join(os.homedir(), ".config/solana/id.json")));
  });

  it("detects .json files as hot wallet", () => {
    assert.ok(isHotWallet("/some/path/key.json"));
  });
});

describe("loadKeypair", () => {
  it("loads a valid keypair file", () => {
    const kp = Keypair.generate();
    const tmpFile = path.join(os.tmpdir(), `rtp-test-kp-${Date.now()}.json`);
    fs.writeFileSync(tmpFile, JSON.stringify(Array.from(kp.secretKey)));
    try {
      const loaded = loadKeypair(tmpFile);
      assert.equal(loaded.publicKey.toBase58(), kp.publicKey.toBase58());
    } finally {
      fs.unlinkSync(tmpFile);
    }
  });

  it("throws for missing file", () => {
    assert.throws(
      () => loadKeypair("/nonexistent/path/key.json"),
      /Keypair file not found/,
    );
  });

  it("throws for invalid JSON", () => {
    const tmpFile = path.join(os.tmpdir(), `rtp-test-kp-bad-${Date.now()}.json`);
    fs.writeFileSync(tmpFile, "not json");
    try {
      assert.throws(
        () => loadKeypair(tmpFile),
        /Invalid keypair/,
      );
    } finally {
      fs.unlinkSync(tmpFile);
    }
  });
});
