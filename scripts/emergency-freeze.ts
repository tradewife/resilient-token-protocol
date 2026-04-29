/**
 * RTP Emergency Freeze / Unfreeze — CLI tool for halting treasury operations.
 *
 * Usage:
 *   npx tsx scripts/emergency-freeze.ts freeze    — freeze treasury (halt all operations)
 *   npx tsx scripts/emergency-freeze.ts unfreeze  — unfreeze treasury (resume operations)
 *   npx tsx scripts/emergency-freeze.ts status     — check frozen state (read-only, no signing)
 *
 * Env vars:
 *   KEYPAIR_PATH  — path to authority keypair JSON (default: ~/.config/solana/id.json)
 *   RPC_URL       — Solana RPC endpoint (default: https://api.devnet.solana.com)
 *   MINT          — token mint address (default: demo mint)
 */

import {
  Connection,
  Keypair,
  PublicKey,
} from "@solana/web3.js";
import fs from "fs";
import path from "path";
import os from "os";

const DEMO_MINT = "FumRWMiDf6FCHuGSYJRPYknCD5F2QNgBmbABZsFJ6q5q";
const RPC_URL = process.env.RPC_URL || "https://api.devnet.solana.com";

function loadKeypair(): Keypair {
  const keypairPath = process.env.KEYPAIR_PATH ||
    path.join(os.homedir(), ".config", "solana", "id.json");

  if (!fs.existsSync(keypairPath)) {
    console.error(`Keypair not found: ${keypairPath}`);
    console.error("Set KEYPAIR_PATH env var or create ~/.config/solana/id.json");
    process.exit(1);
  }

  const content = fs.readFileSync(keypairPath, "utf-8");
  const bytes: number[] = JSON.parse(content);
  return Keypair.fromSecretKey(new Uint8Array(bytes));
}

async function main() {
  const command = process.argv[2];
  if (!command || !["freeze", "unfreeze", "status"].includes(command)) {
    console.error("Usage: npx tsx scripts/emergency-freeze.ts <freeze|unfreeze|status>");
    console.error("");
    console.error("  freeze    — Emergency halt. All 15 state-mutating instructions blocked.");
    console.error("  unfreeze  — Resume operations. Authority-gated.");
    console.error("  status    — Read-only check. No signing required.");
    process.exit(1);
  }

  const mintAddress = process.env.MINT || DEMO_MINT;
  const connection = new Connection(RPC_URL, "confirmed");
  const mint = new PublicKey(mintAddress);

  // Dynamic import for the SDK (ESM/CJS compat).
  const sdk = await import("../sdk/index.ts");

  // Fetch current state for display (read-only, no signing).
  const state = await sdk.fetchTreasuryState(connection, mint);
  console.log(`\n  RTP Emergency Control`);
  console.log(`  ${"".padEnd(40, "─")}`);
  console.log(`  Mint:     ${mint.toBase58()}`);
  console.log(`  RPC:      ${RPC_URL}`);
  console.log(`  Frozen:   ${state.isFrozen ? "YES ⛔" : "NO ✅"}`);
  console.log(`  Action:   ${command.toUpperCase()}\n`);

  if (command === "status") {
    const frozen = await sdk.isTreasuryFrozen(connection, mint);
    if (frozen) {
      console.log("  ⛔ Treasury is FROZEN — all operations halted.");
    } else {
      console.log("  ✅ Treasury is ACTIVE — operations normal.");
    }
    return;
  }

  // Sign transactions — need the authority keypair.
  const payer = loadKeypair();
  console.log(`  Authority: ${payer.publicKey.toBase58()}\n`);

  if (command === "freeze") {
    console.log("  Freezing treasury...");
    const { signature } = await sdk.freezeTreasury(connection, payer, mint);
    console.log(`\n  ⛔ Treasury FROZEN.`);
    console.log(`  TX: https://explorer.solana.com/tx/${signature}?cluster=devnet\n`);
  }

  if (command === "unfreeze") {
    console.log("  Unfreezing treasury...");
    const { signature } = await sdk.unfreezeTreasury(connection, payer, mint);
    console.log(`\n  ✅ Treasury UNFROZEN — operations resumed.`);
    console.log(`  TX: https://explorer.solana.com/tx/${signature}?cluster=devnet\n`);
  }
}

main().catch((err) => {
  console.error("\n  ❌ Error:", err instanceof Error ? err.message : String(err));
  process.exit(1);
});
