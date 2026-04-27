/**
 * compute_adopter_yield_share.ts
 *
 * Pure TypeScript helper for pro-rata yield attribution.
 *
 * Formula:
 *   adopter_yield_share = (fees_contributed / total_fees_received) * yield_pool
 *
 * Each adopting token gets a share of the yield pool proportional to
 * the fees it contributed relative to the total fees received by the
 * treasury.
 *
 * Usage:
 *   npx ts-node scripts/compute_adopter_yield_share.ts
 */

interface AdopterRecord {
  tokenMint: string;
  feesContributed: bigint;   // lamports
}

interface AttributionInput {
  adopters: AdopterRecord[];
  totalFeesReceived: bigint; // lamports — treasury.total_fees_received_lamports
  yieldPool: bigint;         // lamports — total yield to distribute
}

interface AttributionOutput {
  tokenMint: string;
  feesContributed: bigint;
  sharePercent: number;      // percentage of yield pool (0–100, 2 decimal places)
  yieldLamports: bigint;     // attributed yield in lamports
}

/**
 * Compute pro-rata yield attribution for each adopter.
 *
 * Uses integer arithmetic (bigint) to avoid floating-point precision issues.
 * Remainder lamports (from integer division) are distributed one-per-adopter
 * in order until exhausted — consistent with the on-chain redistribution
 * approach.
 */
export function computeAdopterYieldShares(input: AttributionInput): AttributionOutput[] {
  const { adopters, totalFeesReceived, yieldPool } = input;

  if (totalFeesReceived === 0n || yieldPool === 0n) {
    return adopters.map((a) => ({
      tokenMint: a.tokenMint,
      feesContributed: a.feesContributed,
      sharePercent: 0,
      yieldLamports: 0n,
    }));
  }

  // Compute base allocation per adopter
  let totalAllocated = 0n;
  const results = adopters.map((a) => {
    const share = (a.feesContributed * yieldPool) / totalFeesReceived;
    totalAllocated += share;
    const pct = Number((a.feesContributed * 10000n) / totalFeesReceived) / 100;
    return {
      tokenMint: a.tokenMint,
      feesContributed: a.feesContributed,
      sharePercent: pct,
      yieldLamports: share,
    };
  });

  // Distribute remainder (at most N-1 lamports) to first adopters
  let remainder = yieldPool - totalAllocated;
  for (let i = 0; i < results.length && remainder > 0n; i++) {
    results[i].yieldLamports += 1n;
    remainder -= 1n;
  }

  return results;
}

// ── Demo ──

if (typeof require !== "undefined" && require.main === module) {
  const demo: AttributionInput = {
    adopters: [
      { tokenMint: "TokenA_Mint...", feesContributed: 600_000_000_000n },  // 600 SOL
      { tokenMint: "TokenB_Mint...", feesContributed: 400_000_000_000n },  // 400 SOL
    ],
    totalFeesReceived: 1_000_000_000_000n,  // 1000 SOL
    yieldPool: 50_000_000_000n,             // 50 SOL yield
  };

  const shares = computeAdopterYieldShares(demo);
  console.log("Pro-rata Yield Attribution:");
  console.log("─".repeat(60));
  for (const s of shares) {
    console.log(
      `  ${s.tokenMint}: ${s.sharePercent.toFixed(2)}% → ${Number(s.yieldLamports / 1_000_000_000n).toFixed(4)} SOL`
    );
  }
}
