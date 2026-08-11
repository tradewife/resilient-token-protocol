#!/usr/bin/env npx tsx
//
// RTP Operator CLI — Entry Point
//
// Usage:
//   npx tsx cli/bin/rtp.ts <command> [options]
//   npx tsx cli/bin/rtp.ts --help
//

import { createProgram } from "../src/index.js";
import { printError } from "../src/errors.js";

const program = createProgram();

program.parseAsync(process.argv).catch((err) => {
  // Commander throws on --help with code 'commander.helpDisplayed' — that's not an error.
  if (err?.code === "commander.helpDisplayed" || err?.code === "commander.version") {
    process.exit(0);
  }
  printError(err);
  process.exit(typeof err?.code === "number" ? err.code : 1);
});
