/**
 * RTP Redistribute Crank — Permissionless 70/20/10 redistribution trigger.
 *
 * v2: reads on-chain wallet addresses for correct constraint validation.
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
  SystemProgram,
} from "@solana/web3.js";
import fs from "fs";
import path from "path";
import os from "os";

// ─── Configuration ──────────────────────────────────────────────────────

const DEMO_AUTHORITY = new PublicKey("Driyi8Sw2622yCefU34zrjBsQynrDoGD31tBecXrEF6R");

const RPC_URL = process.env.RPC_URL || "https://api.devnet.solana.com";
const JITTER_MAX_MS = parseInt(process.env.JITTER_MAX_MS || "1800000", 10); // 30 min
const AUTHORITY = process.env.AUTHORITY || DEMO_AUTHORITY.toBase58();
const RPC_RETRY_ATTEMPTS = parseInt(process.env.RPC_RETRY_ATTEMPTS || "4", 10);

/** True for devnet/public-RPC blips — cron should exit 0, not CRASHED. */
function isTransientRpcError(err: unknown): boolean {
  const msg = err instanceof Error ? err.message : String(err);
  const logs = (err as { logs?: string[] })?.logs?.join(" ") || "";
  const fullStr = JSON.stringify(err, Object.getOwnPropertyNames(err as object));
  const combined = `${msg} ${logs} ${fullStr}`.toLowerCase();
  return (
    combined.includes("fetch failed") ||
    combined.includes("econnrefused") ||
    combined.includes("econnreset") ||
    combined.includes("etimedout") ||
    combined.includes("socket hang up") ||
    combined.includes("429") ||
    combined.includes("too many requests") ||
    combined.includes("503") ||
    combined.includes("502") ||
    combined.includes("504") ||
    combined.includes("gateway timeout") ||
    combined.includes("rate limit")
  );
}

async function withRpcRetry<T>(label: string, fn: () => Promise<T>): Promise<T> {
  let last: unknown;
  for (let attempt = 1; attempt <= RPC_RETRY_ATTEMPTS; attempt++) {
    try {
      return await fn();
    } catch (err) {
      last = err;
      if (!isTransientRpcError(err) || attempt === RPC_RETRY_ATTEMPTS) {
        throw err;
      }
      const waitMs = 2000 * Math.pow(2, attempt - 1);
      const detail = err instanceof Error ? err.message : String(err);
      console.log(
        `[REDISTRIBUTE] RPC ${label} attempt ${attempt}/${RPC_RETRY_ATTEMPTS} failed (${detail.slice(0, 120)}), retry in ${waitMs}ms`,
      );
      await new Promise((resolve) => setTimeout(resolve, waitMs));
    }
  }
  throw last;
}

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

  const solBalance = await withRpcRetry("getBalance", () =>
    connection.getBalance(payer.publicKey),
  );
  if (solBalance < 5000) {
    throw new Error(`Insufficient SOL for gas: ${(solBalance / 1e9).toFixed(4)} SOL`);
  }

  if (dryRun) {
    console.log("[REDISTRIBUTE] Dry run — no transaction sent.");
    return { redistributeSig: undefined };
  }

  // Fetch on-chain treasury to read the correct wallet addresses.
  // The program validates holders_wallet/project_dev_wallet/ecosystem_wallet
  // against what's stored in the treasury account.
  const { BorshCoder, AnchorProvider, Program } = await import("@coral-xyz/anchor");
  const { IDL } = await import("../sdk/idl.ts");
  const idl = JSON.parse(JSON.stringify(IDL));
  idl.address = "8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB";
  const typeMap: Record<string, any> = {};
  for (const t of idl.types || []) typeMap[t.name] = t;
  for (const acc of idl.accounts || []) {
    if (!acc.type && typeMap[acc.name]) acc.type = typeMap[acc.name].type;
  }

  const authorityPk = typeof authority === "string" ? new PublicKey(authority) : authority;
  const [treasuryPDA] = PublicKey.findProgramAddressSync(
    [Buffer.from("treasury"), authorityPk.toBuffer()],
    new PublicKey(idl.address)
  );

  const coder = new BorshCoder(idl);
  const accountInfo = await withRpcRetry("getAccountInfo(treasury)", () =>
    connection.getAccountInfo(treasuryPDA),
  );
  if (!accountInfo) {
    console.log("[REDISTRIBUTE] Treasury account not found on-chain.");
    return { redistributeSig: undefined };
  }
  const treasuryData = coder.accounts.decode("Treasury", accountInfo.data);

  const wallet = {
    publicKey: payer.publicKey,
    signTransaction: async <T extends any>(tx: T): Promise<T> => {
      (tx as any).partialSign(payer);
      return tx;
    },
    signAllTransactions: async <T extends any>(txs: T[]): Promise<T[]> => {
      return txs.map(tx => { (tx as any).partialSign(payer); return tx; });
    },
  };
  const provider = new AnchorProvider(connection, wallet as any, { commitment: "confirmed" });
  const program = new Program(idl, provider);

  try {
    const tx = await program.methods
      .checkRedistribute()
      .accounts({
        treasury: treasuryPDA,
        holdersWallet: treasuryData.holders_wallet,
        projectDevWallet: treasuryData.project_dev_wallet,
        ecosystemWallet: treasuryData.ecosystem_wallet,
        systemProgram: SystemProgram.programId,
      })
      .transaction();

    const { Transaction: Tx, sendAndConfirmTransaction: sendConfirm } = await import("@solana/web3.js");
    const { blockhash, lastValidBlockHeight } = await withRpcRetry("getLatestBlockhash", () =>
      connection.getLatestBlockhash(),
    );
    tx.recentBlockhash = blockhash;
    tx.lastValidBlockHeight = lastValidBlockHeight;
    tx.feePayer = payer.publicKey;
    tx.partialSign(payer);
    const rawTx = tx.serialize();
    const sig = await withRpcRetry("sendRawTransaction", () =>
      connection.sendRawTransaction(rawTx),
    );
    await withRpcRetry("confirmTransaction", () =>
      connection.confirmTransaction({ signature: sig, blockhash, lastValidBlockHeight }, "confirmed"),
    );

    return { redistributeSig: sig };
  } catch (err: unknown) {
    // Normalize error to string for pattern matching — Solana/Anchor errors are deeply nested
    const msg = err instanceof Error ? err.message : String(err);
    const logs = (err as any)?.logs?.join(" ") || "";
    // Stringify the full error object as a last resort — Anchor wraps errors in ways that
    // .message alone doesn't contain the useful patterns (e.g. AccountNotInitialized)
    const fullStr = JSON.stringify(err, Object.getOwnPropertyNames(err));
    const combined = `${msg} ${logs} ${fullStr}`;
    if (combined.includes("BelowThreshold") || combined.includes("InsufficientRunway")) {
      console.log("[REDISTRIBUTE] Below threshold — no redistribution triggered.");
      return { redistributeSig: undefined };
    }
    // Devnet BPF loader cache bug: old binary may still reference removed accounts (e.g. "mint")
    // Catch gracefully so Railway doesn't mark the service as Crashed.
    if (combined.includes("AccountNotInitialized") || combined.includes("AccountOwnedByWrongProgram") || combined.includes("account: mint") || combined.includes("custom program error: 0xbc4")) {
      console.log("[REDISTRIBUTE] On-chain program binary stale (devnet BPF cache). Skipping. Error:", msg.substring(0, 200));
      return { redistributeSig: undefined };
    }
    if (isTransientRpcError(err)) {
      console.log("[REDISTRIBUTE] Transient RPC failure after retries — skipping this cycle.");
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
    const msg = err instanceof Error ? err.message : String(err);
    // Graceful exit for known devnet issues — don't mark service as Crashed
    const fullStr = JSON.stringify(err, Object.getOwnPropertyNames(err));
    const combined = `${msg} ${fullStr}`;
    if (combined.includes("AccountNotInitialized") || combined.includes("custom program error: 0xbc4")) {
      console.log("[REDISTRIBUTE] On-chain program binary stale (devnet BPF cache). Graceful exit.");
      process.exit(0);
    }
    if (isTransientRpcError(err)) {
      console.log("[REDISTRIBUTE] Transient RPC failure — graceful exit (will retry next cron).");
      process.exit(0);
    }
    console.error("[REDISTRIBUTE] Fatal:", msg);
    process.exit(1);
  });
}
