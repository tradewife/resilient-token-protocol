/**
 * Live trade PnL helpers.
 *
 * Historical bug: the dashboard subtracted Flash v1 documented/listed fees
 *   feeDrag = 0.12 + 0.0042 * hold_hours
 * from every closed trade. That model is ~5× too expensive vs measured
 * venue costs and produced a fake "net mainnet" figure around −2.56%.
 *
 * Current venue (GMTrade, Aug 2026 probe): round-trip order fee ≈ 0.022%
 * of notional. Borrow is skip-smaller-side and small on short holds; we
 * do not invent a per-hour borrow term here until the trader exposes it.
 *
 * `pnl_pct` from rtp-trader is price PnL in percent of notional (side-correct).
 * Net = price PnL − measured RT fee (flat bps of notional, both sides).
 */

export const GMTRADE_RT_FEE_PCT = 0.022; // 0.022% of notional, measured

/**
 * Equity exposure per round trip: 20% of wallet committed as collateral
 * (`RTP_TRADER_POSITION_FRACTION`) × 9× leverage (`RTP_TRADER_LEVERAGE`)
 * = 1.8× equity at risk per trade. Per-trade equity growth factor is
 * `1 + exposure × net%/100`; the headline curve compounds those factors.
 * This is the capital-growth number — the unweighted sum of per-trade %
 * is still exposed (`totalNetPct`) for the tape, but is NOT a return.
 */
export const EQUITY_EXPOSURE_PER_TRADE = 0.2 * 9;

export type ClosedTradeLike = {
  entry_price: number;
  exit_price: number;
  entry_time: number;
  exit_time: number;
  pnl_pct: number;
  size_usd?: number;
  side?: string;
  exit_reason?: string;
};

export function holdHours(t: Pick<ClosedTradeLike, "entry_time" | "exit_time">): number {
  return Math.max(0, (t.exit_time - t.entry_time) / 3600);
}

/** Flat measured round-trip fee in percentage points of notional. */
export function measuredRtFeePct(_t?: ClosedTradeLike): number {
  return GMTRADE_RT_FEE_PCT;
}

/** Per-trade net PnL % after measured venue RT fee.
 *
 * PhantomClear rows are reconciliation audit entries (positions closed
 * outside the trader's observation), not real round trips — subtracting the
 * venue fee from their estimated PnL would invent a cost that was never
 * charged to this accounting. They pass through gross.
 */
export function netTradePnlPct(t: ClosedTradeLike): number {
  if (t.exit_reason?.startsWith("PhantomClear")) {
    return t.pnl_pct;
  }
  return t.pnl_pct - measuredRtFeePct(t);
}

/** Gross price PnL % (as stored by the trader). */
export function grossTradePnlPct(t: ClosedTradeLike): number {
  return t.pnl_pct;
}

export type PnlSeries = {
  /** Cumulative sum of per-trade net PnL % (unweighted — tape bookkeeping, NOT a return). */
  totalNetPct: number;
  /** Cumulative sum of per-trade gross price PnL %. */
  totalGrossPct: number;
  /** Compounded equity return % — headline figure. Compounds per-trade
   * net % at the actual capital exposure (20% wallet × 9× = 1.8×). */
  totalEquityPct: number;
  /** Per-trade net series (same order as input). */
  netTrades: number[];
  /** Running cumulative net for sparkline (starts at 0) — unweighted sum. */
  cumulativeNet: number[];
  /** Running compounded equity return % (starts at 0). */
  cumulativeEquity: number[];
  tradeCount: number;
  winRatePct: number | null;
};

export function summarizeTradePnl(trades: ClosedTradeLike[] | null | undefined): PnlSeries {
  if (!trades || trades.length === 0) {
    return {
      totalNetPct: 0,
      totalGrossPct: 0,
      totalEquityPct: 0,
      netTrades: [],
      cumulativeNet: [],
      cumulativeEquity: [],
      tradeCount: 0,
      winRatePct: null,
    };
  }

  const netTrades = trades.map(netTradePnlPct);
  const totalNetPct = netTrades.reduce((a, b) => a + b, 0);
  const totalGrossPct = trades.reduce((a, t) => a + t.pnl_pct, 0);
  const cumulativeNet: number[] = [0];
  const cumulativeEquity: number[] = [0];
  let acc = 0;
  let equity = 1.0;
  for (const n of netTrades) {
    acc += n;
    cumulativeNet.push(acc);
    equity *= 1 + EQUITY_EXPOSURE_PER_TRADE * (n / 100);
    cumulativeEquity.push((equity - 1) * 100);
  }
  const wins = netTrades.filter((n) => n > 0).length;
  return {
    totalNetPct,
    totalGrossPct,
    totalEquityPct: (equity - 1) * 100,
    netTrades,
    cumulativeNet,
    cumulativeEquity,
    tradeCount: trades.length,
    winRatePct: (wins / netTrades.length) * 100,
  };
}

export function formatPnlPct(pct: number, digits = 2): string {
  const sign = pct > 0 ? "+" : "";
  return `${sign}${pct.toFixed(digits)}%`;
}
