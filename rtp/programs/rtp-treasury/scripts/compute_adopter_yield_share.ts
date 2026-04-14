/**
 * Computes an adopter's pro-rata yield share given on-chain state.
 *
 * Formula:
 *   adopter_yield_share = (fees_contributed / total_fees_received) * yield_available
 *
 * This is a pure TypeScript function — no on-chain state.
 * It is called by the redistribution logic to determine how much yield
 * each adopting token project receives before holder-level distribution.
 */
export function computeAdopterYieldShare(
  feesContributedLamports: bigint,
  totalFeesReceivedLamports: bigint,
  yieldAvailableLamports: bigint
): bigint {
  if (totalFeesReceivedLamports === 0n) return 0n;
  // Use integer math: multiply first to avoid precision loss
  return (feesContributedLamports * yieldAvailableLamports) / totalFeesReceivedLamports;
}

/**
 * Example: Two adopters, one yield pool
 *
 * TokenA contributed 600 SOL in fees
 * TokenB contributed 400 SOL in fees
 * Total yield available: 100 SOL
 *
 * TokenA share: (600 / 1000) * 100 = 60 SOL → goes to TokenA holder snapshot
 * TokenB share: (400 / 1000) * 100 = 40 SOL → goes to TokenB holder snapshot
 */
export function exampleAttribution() {
  const tokenA = computeAdopterYieldShare(600_000_000_000n, 1_000_000_000_000n, 100_000_000_000n);
  const tokenB = computeAdopterYieldShare(400_000_000_000n, 1_000_000_000_000n, 100_000_000_000n);
  console.log(`TokenA yield share: ${Number(tokenA) / 1e9} SOL`); // 60
  console.log(`TokenB yield share: ${Number(tokenB) / 1e9} SOL`); // 40
}

// Run example when executed directly
exampleAttribution();
