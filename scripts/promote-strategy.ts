/**
 * RTP Strategy Promotion — reads Night Shift results, checks promotion criteria,
 * and calls register_strategy on-chain.
 *
 * Promotion gate (calibrated against SOL/USDT Survivor 2.69):
 *   - OOS Sharpe >= 2.5
 *   - WFA consistency >= 70% (7/9 folds positive)
 *   - Avg trades per fold >= 15
 *   - Fragility <= 0.40
 *
 * Runs as a Railway cron 30 minutes after the night shift completes.
 *
 * Usage:
 *   npx tsx scripts/promote-strategy.ts
 *
 * Env vars:
 *   KEYPAIR_PATH        — path to authority keypair JSON (default: ~/.config/solana/id.json)
 *   RPC_URL             — Solana RPC endpoint (default: https://api.devnet.solana.com)
 *   NIGHT_RESULTS_DIR   — path to data/night_results (default: ./data/night_results)
 *   DRY_RUN             — if "true", evaluates but does not submit on-chain (default: false)
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

const DEMO_MINT = "FumRWMiDf6FCHuGSYJRPYknCD5F2QNgBmbABZsFJ6q5q";

const RPC_URL = process.env.RPC_URL || "https://api.devnet.solana.com";
const NIGHT_RESULTS_DIR = process.env.NIGHT_RESULTS_DIR || "./data/night_results";
const DRY_RUN = process.env.DRY_RUN === "true";

// Promotion gate thresholds
const PROMOTION_GATE = {
  minOosSharpe: 2.5,
  minWfaConsistency: 0.70,
  minTradesPerFold: 15,
  maxFragility: 0.40,
};

// ─── Types ───────────────────────────────────────────────────────────────

interface TopCandidate {
  symbol: string;
  params: Record<string, unknown>;
  survivor_score: number;
  oos_sharpe: number;
  oos_consistency: number;
  oos_max_dd: number;
  overfitting_score: number;
  fragility: number;
  oos_avg_trades_per_fold: number;
  rejected: boolean;
  rejection_reason: string | null;
}

interface SummaryJson {
  date: string;
  symbols: string[];
  top_candidates: TopCandidate[];
}

// ─── Exported API (for CLI import) ──────────────────────────────────────

export interface PromoteStrategyOptions {
  dryRun?: boolean;
  resultsDir?: string;
}

export interface PromoteStrategyResult {
  strategyPDA?: string;
  signature?: string;
  strategyId?: string;
}

/**
 * Programmable strategy promotion — called by `rtp strategy promote`.
 * Reads night shift results, evaluates against promotion gate, submits on-chain.
 */
export async function exportPromoteStrategy(
  connection: Connection,
  payer: Keypair,
  mint: string,
  opts?: PromoteStrategyOptions,
): Promise<PromoteStrategyResult | null> {
  const dryRun = opts?.dryRun ?? false;
  const resultsDir = opts?.resultsDir ?? "./data/night_results";

  const summaryPath = findLatestSummaryInDir(resultsDir);
  if (!summaryPath) return null;

  const summary: SummaryJson = JSON.parse(fs.readFileSync(summaryPath, "utf-8"));
  const eligible = summary.top_candidates.filter(c => !c.rejected);
  if (eligible.length === 0) return null;

  const passing = eligible
    .map(c => ({ candidate: c, evaluation: evaluateCandidate(c) }))
    .filter(({ evaluation }) => evaluation.passed);

  if (passing.length === 0) return null;

  const best = passing.reduce((a, b) =>
    a.candidate.survivor_score > b.candidate.survivor_score ? a : b
  );
  const { candidate } = best;

  const strategyId = makeStrategyId(candidate.symbol, candidate.survivor_score);
  const promotionSharpeX100 = Math.round(candidate.oos_sharpe * 100);

  if (dryRun) {
    return { strategyId, strategyPDA: undefined, signature: undefined };
  }

  const sdk = await import("../sdk/index.ts");
  const result = await sdk.registerStrategy(
    connection,
    payer,
    mint,
    strategyId,
    promotionSharpeX100,
  );

  return {
    strategyPDA: result.strategyPDA,
    signature: result.signature,
    strategyId,
  };
}

function findLatestSummaryInDir(resultsDir: string): string | null {
  if (!fs.existsSync(resultsDir)) return null;
  const dateDirs = fs.readdirSync(resultsDir)
    .filter(d => /^\d{4}-\d{2}-\d{2}$/.test(d))
    .sort()
    .reverse();
  for (const dateDir of dateDirs) {
    const summaryPath = path.join(resultsDir, dateDir, "summary.json");
    if (fs.existsSync(summaryPath)) return summaryPath;
  }
  return null;
}

// ─── Helpers ─────────────────────────────────────────────────────────────

function banner() {
  console.log(`\n${"═".repeat(60)}`);
  console.log(`  RTP Strategy Promotion — ${new Date().toISOString()}`);
  console.log(`${"═".repeat(60)}\n`);
}

function loadKeypair(): Keypair {
  const keypairPath = process.env.KEYPAIR_PATH ||
    path.join(os.homedir(), ".config", "solana", "id.json");

  if (!fs.existsSync(keypairPath)) {
    console.error(`[PROMOTE] Keypair not found: ${keypairPath}`);
    process.exit(1);
  }

  const content = fs.readFileSync(keypairPath, "utf-8");
  const bytes: number[] = JSON.parse(content);
  return Keypair.fromSecretKey(new Uint8Array(bytes));
}

/**
 * Find the latest summary.json in data/night_results/YYYY-MM-DD/summary.json
 */
function findLatestSummary(): string | null {
  if (!fs.existsSync(NIGHT_RESULTS_DIR)) {
    return null;
  }

  const dateDirs = fs.readdirSync(NIGHT_RESULTS_DIR)
    .filter(d => /^\d{4}-\d{2}-\d{2}$/.test(d))
    .sort()
    .reverse(); // newest first

  for (const dateDir of dateDirs) {
    const summaryPath = path.join(NIGHT_RESULTS_DIR, dateDir, "summary.json");
    if (fs.existsSync(summaryPath)) {
      return summaryPath;
    }
  }

  return null;
}

/**
 * Evaluate a candidate against the promotion gate.
 * Returns null if rejected, or the candidate with gate results.
 */
function evaluateCandidate(candidate: TopCandidate): {
  passed: boolean;
  reasons: string[];
} {
  const reasons: string[] = [];

  if (candidate.rejected) {
    return { passed: false, reasons: [`Night shift rejected: ${candidate.rejection_reason}`] };
  }

  if (candidate.oos_sharpe < PROMOTION_GATE.minOosSharpe) {
    reasons.push(`OOS Sharpe ${candidate.oos_sharpe.toFixed(2)} < ${PROMOTION_GATE.minOosSharpe}`);
  }

  if (candidate.oos_consistency < PROMOTION_GATE.minWfaConsistency) {
    reasons.push(`WFA consistency ${(candidate.oos_consistency * 100).toFixed(0)}% < ${(PROMOTION_GATE.minWfaConsistency * 100).toFixed(0)}%`);
  }

  if (candidate.oos_avg_trades_per_fold < PROMOTION_GATE.minTradesPerFold) {
    reasons.push(`Avg trades/fold ${candidate.oos_avg_trades_per_fold.toFixed(1)} < ${PROMOTION_GATE.minTradesPerFold}`);
  }

  if (candidate.fragility > PROMOTION_GATE.maxFragility) {
    reasons.push(`Fragility ${candidate.fragility.toFixed(3)} > ${PROMOTION_GATE.maxFragility}`);
  }

  return { passed: reasons.length === 0, reasons };
}

/**
 * Generate a short strategy ID from the symbol and survivor score.
 * Must be 1-16 chars.
 */
function makeStrategyId(symbol: string, score: number): string {
  // e.g. "SOL_2.69" from SOL/USDT, score 2.69
  const base = symbol.split("/")[0]; // SOL from SOL/USDT
  const id = `${base}_${score.toFixed(1)}`;
  if (id.length > 16) {
    return id.substring(0, 16);
  }
  return id;
}

// ─── Main ───────────────────────────────────────────────────────────────

async function main() {
  banner();

  console.log(`[PROMOTE] RPC: ${RPC_URL}`);
  console.log(`[PROMOTE] Mint: ${DEMO_MINT}`);
  console.log(`[PROMOTE] Results dir: ${NIGHT_RESULTS_DIR}`);
  console.log(`[PROMOTE] DRY_RUN: ${DRY_RUN}`);
  console.log(`[PROMOTE] Gate: Sharpe>=${PROMOTION_GATE.minOosSharpe}, Consistency>=${(PROMOTION_GATE.minWfaConsistency * 100).toFixed(0)}%, Trades/fold>=${PROMOTION_GATE.minTradesPerFold}, Fragility<=${PROMOTION_GATE.maxFragility}`);

  // Step 1: Find latest night shift results.
  const summaryPath = findLatestSummary();
  if (!summaryPath) {
    console.log("[PROMOTE] No night shift results found. Nothing to promote.");
    console.log("[PROMOTE] exit 0 (no results is not an error)");
    return;
  }
  console.log(`[PROMOTE] Latest results: ${summaryPath}`);

  const summary: SummaryJson = JSON.parse(fs.readFileSync(summaryPath, "utf-8"));
  console.log(`[PROMOTE] Date: ${summary.date}, Symbols: ${summary.symbols.join(", ")}`);
  console.log(`[PROMOTE] Candidates: ${summary.top_candidates.length}`);

  if (summary.top_candidates.length === 0) {
    console.log("[PROMOTE] No candidates in night shift results. Nothing to promote.");
    return;
  }

  // Step 2: Evaluate each non-rejected candidate.
  const eligible = summary.top_candidates.filter(c => !c.rejected);
  console.log(`[PROMOTE] Eligible (not rejected): ${eligible.length}`);

  if (eligible.length === 0) {
    console.log("[PROMOTE] All candidates were rejected by night shift. Nothing to promote.");
    return;
  }

  // Evaluate against promotion gate.
  for (const candidate of eligible) {
    const eval_ = evaluateCandidate(candidate);
    const status = eval_.passed ? "PASS" : "FAIL";
    console.log(`[PROMOTE] ${status}: ${candidate.symbol} — Sharpe=${candidate.oos_sharpe.toFixed(2)}, ` +
      `Cons=${(candidate.oos_consistency * 100).toFixed(0)}%, ` +
      `Trades/fold=${candidate.oos_avg_trades_per_fold.toFixed(1)}, ` +
      `Fragility=${candidate.fragility.toFixed(3)}`);
    if (!eval_.passed) {
      for (const reason of eval_.reasons) {
        console.log(`[PROMOTE]   ❌ ${reason}`);
      }
    }
  }

  // Step 3: Pick the best passing candidate (highest survivor_score).
  const passing = eligible
    .map(c => ({ candidate: c, evaluation: evaluateCandidate(c) }))
    .filter(({ evaluation }) => evaluation.passed);

  if (passing.length === 0) {
    console.log("\n[PROMOTE] No candidates passed the promotion gate. Nothing to promote.");
    console.log("[PROMOTE] exit 0 (no qualifying strategy is not an error)");
    return;
  }

  const best = passing.reduce((a, b) =>
    a.candidate.survivor_score > b.candidate.survivor_score ? a : b
  );
  const { candidate } = best;

  console.log(`\n[PROMOTE] Best candidate: ${candidate.symbol}`);
  console.log(`[PROMOTE]   Survivor score: ${candidate.survivor_score.toFixed(4)}`);
  console.log(`[PROMOTE]   OOS Sharpe: ${candidate.oos_sharpe.toFixed(2)}`);
  console.log(`[PROMOTE]   Consistency: ${(candidate.oos_consistency * 100).toFixed(0)}%`);
  console.log(`[PROMOTE]   Fragility: ${candidate.fragility.toFixed(3)}`);
  console.log(`[PROMOTE]   Params: ${JSON.stringify(candidate.params)}`);

  const strategyId = makeStrategyId(candidate.symbol, candidate.survivor_score);
  const promotionSharpeX100 = Math.round(candidate.oos_sharpe * 100);

  console.log(`[PROMOTE] Strategy ID: ${strategyId}`);
  console.log(`[PROMOTE] Promotion Sharpe (x100): ${promotionSharpeX100}`);

  if (DRY_RUN) {
    console.log("\n[PROMOTE] DRY RUN — would call register_strategy with:");
    console.log(`[PROMOTE]   mint: ${DEMO_MINT}`);
    console.log(`[PROMOTE]   strategy_id: "${strategyId}"`);
    console.log(`[PROMOTE]   promotion_sharpe_x100: ${promotionSharpeX100}`);
    console.log("[PROMOTE] exit 0 (dry run)");
    return;
  }

  // Step 4: Connect and submit on-chain.
  const connection = new Connection(RPC_URL, "confirmed");
  const payer = loadKeypair();

  console.log(`[PROMOTE] Payer: ${payer.publicKey.toBase58()}`);

  const solBalance = await connection.getBalance(payer.publicKey);
  console.log(`[PROMOTE] Payer SOL: ${(solBalance / 1e9).toFixed(4)} SOL`);
  if (solBalance < 5000) {
    console.error("[PROMOTE] Payer has insufficient SOL for gas. Exiting.");
    process.exit(1);
  }

  try {
    const sdk = await import("../sdk/index.ts");

    console.log("\n[PROMOTE] Calling registerStrategy...");

    const result = await sdk.registerStrategy(
      connection,
      payer,
      DEMO_MINT,
      strategyId,
      promotionSharpeX100,
    );

    console.log(`[PROMOTE] Strategy promoted!`);
    console.log(`[PROMOTE]   Strategy PDA: ${result.strategyPDA}`);
    console.log(`[PROMOTE]   TX: ${result.signature}`);
    console.log(`[PROMOTE]   Explorer: https://explorer.solana.com/tx/${result.signature}?cluster=devnet`);
    console.log("\n[PROMOTE] ✅ Promotion complete.");
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err);

    // StrategyAlreadyExists — if the same strategy_id was already registered
    if (msg.includes("already in use") || msg.includes("already exists") || msg.includes("0x0") || msg.includes("custom program error: 0x0")) {
      console.log(`[PROMOTE] Strategy "${strategyId}" already registered. Skipping.`);
      console.log("[PROMOTE] ✅ No action needed (idempotent).");
      return;
    }

    console.error(`[PROMOTE] ❌ Error: ${msg}`);

    const anchorErr = err as { error?: { errorCode?: { code?: string; number?: number }; errorMessage?: string } };
    if (anchorErr.error) {
      console.error(`[PROMOTE] Anchor error: ${anchorErr.error.errorCode?.code} (${anchorErr.error.errorCode?.number})`);
      console.error(`[PROMOTE] Message: ${anchorErr.error.errorMessage}`);
    }

    process.exit(1);
  }
}

// Guard: only run main() when executed directly, not when imported
const isDirectRun = typeof require !== "undefined"
  ? require.main === module
  : process.argv[1]?.includes("promote-strategy");
if (isDirectRun) {
  main().catch((err) => {
    console.error("[PROMOTE] Fatal:", err);
    process.exit(1);
  });
}
