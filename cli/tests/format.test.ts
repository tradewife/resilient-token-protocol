// Tests for output formatting.

import { describe, it } from "node:test";
import assert from "node:assert/strict";

import { getOutputMode, type OutputMode } from "../src/format.js";

describe("getOutputMode", () => {
  it("returns json when --json flag is set", () => {
    assert.equal(getOutputMode({ json: true }), "json");
  });

  it("returns quiet when --quiet flag is set", () => {
    assert.equal(getOutputMode({ quiet: true }), "quiet");
  });

  it("returns human by default", () => {
    assert.equal(getOutputMode({}), "human");
  });

  it("json takes precedence over quiet", () => {
    assert.equal(getOutputMode({ json: true, quiet: true }), "json");
  });
});
