/**
 * RTP Redistribute Crank — Permissionless 70/20/10 redistribution trigger.
 *
 * Runs as a Railway cron service:
 *   1. Random jitter delay (0–30 min) to avoid predictable timing
 *   2. Call checkRedistribute for the configured authority treasury
 *   3. Log results
 *
 * Usage:
 *   npx tsx scripts/redistribute.ts
 *
 * Env vars:
 *   KEYPAIR_PATH  — path to fee-payer keypair JSON (default: ~/.config/solana/id.json)
 *   RPC_URL       — Solana RPC endpoint (default: https://api.devnet.solana.com)
 *   AUTHORITY     — treasury authority pubkey (default: DEMO_AUTHORITY)
 *   JITTER_MAX_MS — max jitter in ms (default: 1800000 = 30 min)
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

const DEMO_AUTHORITY = new PublicKey("3yMH4kCBk9vNHLU6gqqNn125rmzTSSpJP8FiLXDtaEH5");

const RPC_URL = process.env.RPC_URL || "https://api.devnet.solana.com";
const JITTER_MAX_MS = parseInt(process.env.JITTER_MAX_MS || "1800000", 10); // 30 min
const AUTHORITY = process.env.AUTHORITY || DEMO_AUTHORITY.toBase58();

// ─── Exported API (for CLI import) ─────────────────────────────────────

export interface RedistributeOptions {
  dryRun?: boolean;
  jitterMaxMs?: number;
}

export interface RedistributeResult {
  redistributeSig?: string;
}

export async function exportRedistribute(
  connection: Connection,
  payer: Keypair,
  authority: PublicKey,
  opts?: RedistributeOptions,
): Promise<RedistributeResult> {
  const dryRun = opts?.dryRun ?? false;
  const jitterMaxMs = opts?.jitterMaxMs ?? 0;

  if (jitterMaxMs > 0) {
    const jitter = Math.floor(Math.random() * jitterMaxMs);
    await new Promise((resolve) => setTimeout(resolve, jitter));
  }

  const solBalance = await connection.getBalance(payer.publicKey);
  if (solBalance < 5000) {
    throw new Error(`Insufficient SOL for gas: ${(solBalance / 1e9).toFixed(4)} SOL`);
  }

  if (dryRun) {
    console.log("[REDISTRIBUTE] Dry run — no transaction sent.");
    return { redistributeSig: undefined };
  }

  const sdk = await import("../sdk/index.ts");

  try {
    const result = await sdk.checkRedistribute(connection, payer, authority);
    return {
      redistributeSig: result.redistributeSig,
    };
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err);
    if (msg.includes("BelowThreshold") || msg.includes("InsufficientRunway")) {
      console.log("[REDISTRIBUTE] Below threshold — no redistribution triggered.");
      return { redistributeSig: undefined };
    }
    // Devnet BPF loader cache bug: old binary may still reference removed accounts (e.g. "mint")
    // Catch gracefully so Railway doesn't mark the service as Crashed.
    if (msg.includes("AccountNotInitialized") || msg.includes("AccountOwnedByWrongProgram") || msg.includes("mint")) {
      console.log("[REDISTRIBUTE] On-chain program binary stale (devnet BPF cache). Skipping. Error:", msg.substring(0, 200));
      return { redistributeSig: undefined };
    }
    throw err;
  }
}

// ─── Standalone Runner ──────────────────────────────────────────────────

function banner() {
  console.log(`\n${"═".repeat(60)}`);
  console.log(`  RTP Redistribute Crank — ${new Date().toISOString()}`);
  console.log(`${"═".repeat(60)}\n`);
}

function loadKeypair(): Keypair {
  const keypairPath = process.env.KEYPAIR_PATH ||
    path.join(os.homedir(), ".config", "solana", "id.json");

  if (!fs.existsSync(keypairPath)) {
    console.error(`[REDISTRIBUTE] Keypair not found: ${keypairPath}`);
    console.error("[REDISTRIBUTE] Set KEYPAIR_PATH env var or create ~/.config/solana/id.json");
    process.exit(1);
  }

  const content = fs.readFileSync(keypairPath, "utf-8");
  const bytes: number[] = JSON.parse(content);
  return Keypair.fromSecretKey(new Uint8Array(bytes));
}

async function main() {
  banner();

  if (JITTER_MAX_MS > 0) {
    const jitter = Math.floor(Math.random() * JITTER_MAX_MS);
    console.log(`[REDISTRIBUTE] Jitter delay: ${(jitter / 1000 / 60).toFixed(1)} min`);
    await new Promise((resolve) => setTimeout(resolve, jitter));
  }

  const connection = new Connection(RPC_URL, "confirmed");
  const payer = loadKeypair();
  const authority = new PublicKey(AUTHORITY);

  console.log(`[REDISTRIBUTE] Authority: ${authority.toBase58()}`);
  console.log(`[REDISTRIBUTE] Payer: ${payer.publicKey.toBase58()}`);
  console.log(`[REDISTRIBUTE] RPC: ${RPC_URL}`);

  const result = await exportRedistribute(connection, payer, authority, {
    jitterMaxMs: 0,  // Jitter already applied above in main()
  });

  if (result.redistributeSig) {
    console.log(`[REDISTRIBUTE] Redistribute TX: https://explorer.solana.com/tx/${result.redistributeSig}?cluster=devnet`);
    console.log("[REDISTRIBUTE] SUCCESS");
  } else {
    console.log("[REDISTRIBUTE] No redistribution triggered (below threshold or treasury insufficient).");
  }
}

// Guard: only run main() when executed directly
if (typeof require !== "undefined" && require.main === module) {
  main().catch((err) => {
    console.error("[REDISTRIBUTE] Fatal:", err instanceof Error ? err.message : String(err));
    process.exit(1);
  });
}
