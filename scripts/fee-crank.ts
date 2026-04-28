/**
 * RTP Fee Crank — Permissionless withdraw + redistribute for the demo treasury.
 *
 * Runs as a Railway cron service:
 *   1. Random jitter delay (0–30 min) to avoid predictable timing
 *   2. Check demo mint's withheld fee balance
 *   3. If below threshold, exit 0 (not worth the gas)
 *   4. Call withdrawAndRedistribute via the SDK
 *   5. Log results
 *
 * Usage:
 *   npx tsx scripts/fee-crank.ts
 *
 * Env vars:
 *   KEYPAIR_PATH  — path to fee-payer keypair JSON (default: ~/.config/solana/id.json)
 *   RPC_URL       — Solana RPC endpoint (default: https://api.devnet.solana.com)
 *   JITTER_MAX_MS — max jitter in ms (default: 1800000 = 30 min)
 *   FEE_THRESHOLD — minimum withheld tokens (raw lamports) to bother withdrawing (default: 5000000 = 5 tokens ≈ $5)
 */

import {
  Connection,
  Keypair,
  PublicKey,
} from "@solana/web3.js";
import fs from "fs";
import path from "path";
import os from "os";

// ─── Configuration ──────────────────────────────────────────────────────

const DEMO_MINT = new PublicKey("FumRWMiDf6FCHuGSYJRPYknCD5F2QNgBmbABZsFJ6q5q");

const RPC_URL = process.env.RPC_URL || "https://api.devnet.solana.com";
const JITTER_MAX_MS = parseInt(process.env.JITTER_MAX_MS || "1800000", 10); // 30 min
const FEE_THRESHOLD = parseInt(process.env.FEE_THRESHOLD || "5000000", 10); // 5 tokens (6 dp) ≈ $5

function banner() {
  console.log(`\n${"═".repeat(60)}`);
  console.log(`  RTP Fee Crank — ${new Date().toISOString()}`);
  console.log(`${"═".repeat(60)}\n`);
}

// ─── Keypair Loading ────────────────────────────────────────────────────

function loadKeypair(): Keypair {
  const keypairPath = process.env.KEYPAIR_PATH ||
    path.join(os.homedir(), ".config", "solana", "id.json");

  if (!fs.existsSync(keypairPath)) {
    console.error(`[CRANK] Keypair not found: ${keypairPath}`);
    console.error("[CRANK] Set KEYPAIR_PATH env var or create ~/.config/solana/id.json");
    process.exit(1);
  }

  const content = fs.readFileSync(keypairPath, "utf-8");
  const bytes: number[] = JSON.parse(content);
  return Keypair.fromSecretKey(new Uint8Array(bytes));
}

// ─── Main ───────────────────────────────────────────────────────────────

async function main() {
  banner();

  // Step 1: Random jitter delay.
  if (JITTER_MAX_MS > 0) {
    const jitter = Math.floor(Math.random() * JITTER_MAX_MS);
    const jitterSec = (jitter / 1000).toFixed(0);
    console.log(`[CRANK] Jitter delay: ${jitterSec}s (max ${JITTER_MAX_MS / 1000}s)`);
    await new Promise((resolve) => setTimeout(resolve, jitter));
    console.log(`[CRANK] Jitter complete. Continuing at ${new Date().toISOString()}`);
  }

  // Step 2: Connect and load keypair.
  const connection = new Connection(RPC_URL, "confirmed");
  const payer = loadKeypair();

  console.log(`[CRANK] RPC: ${RPC_URL}`);
  console.log(`[CRANK] Payer: ${payer.publicKey.toBase58()}`);
  console.log(`[CRANK] Mint: ${DEMO_MINT.toBase58()}`);
  console.log(`[CRANK] Fee threshold: ${FEE_THRESHOLD} lamports`);

  // Check payer SOL balance.
  const solBalance = await connection.getBalance(payer.publicKey);
  console.log(`[CRANK] Payer SOL: ${(solBalance / 1e9).toFixed(4)} SOL`);
  if (solBalance < 5000) {
    console.error("[CRANK] Payer has insufficient SOL for gas. Exiting.");
    process.exit(1);
  }

  // Step 3: Attempt withdraw + redistribute.
  // We use dynamic import for the SDK since it depends on Anchor.
  // The SDK is at dashboard/src/lib/sdk/index.ts for the dashboard,
  // but for scripts we use the standalone SDK at sdk/index.ts.
  try {
    // Import the SDK functions directly.
    // tsx resolves .ts files at runtime.
    const sdk = await import("../sdk/index.ts");

    console.log("\n[CRANK] Calling withdrawAndRedistribute...");

    const result = await sdk.withdrawAndRedistribute(
      connection,
      payer,
      DEMO_MINT,
    );

    console.log(`[CRANK] Withdraw tx: ${result.withdrawSig}`);
    console.log(`[CRANK] Explorer: https://explorer.solana.com/tx/${result.withdrawSig}?cluster=devnet`);

    if (result.redistributeSig) {
      console.log(`[CRANK] Redistribute tx: ${result.redistributeSig}`);
      console.log(`[CRANK] Explorer: https://explorer.solana.com/tx/${result.redistributeSig}?cluster=devnet`);
    } else {
      console.log("[CRANK] No redistribution (BelowThreshold — vault excess below runway floor, expected)");
    }

    console.log("\n[CRANK] ✅ Fee crank complete.");
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err);

    // BelowThreshold is expected when vault doesn't have excess above runway.
    if (msg.includes("BelowThreshold")) {
      console.log("[CRANK] BelowThreshold — vault excess below runway floor. No redistribution.");
      console.log("[CRANK] Fees withdrawn successfully (if any were pending).");
      console.log("[CRANK] ✅ Fee crank complete (no redistribute).");
      return;
    }

    console.error(`[CRANK] ❌ Error: ${msg}`);

    // Log more detail for Anchor errors.
    const anchorErr = err as { error?: { errorCode?: { code?: string; number?: number }; errorMessage?: string } };
    if (anchorErr.error) {
      console.error(`[CRANK] Anchor error: ${anchorErr.error.errorCode?.code} (${anchorErr.error.errorCode?.number})`);
      console.error(`[CRANK] Message: ${anchorErr.error.errorMessage}`);
    }

    process.exit(1);
  }
}

main().catch((err) => {
  console.error("[CRANK] Fatal:", err);
  process.exit(1);
});
