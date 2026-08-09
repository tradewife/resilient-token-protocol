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

export type ClosedTradeLike = {
  entry_price: number;
  exit_price: number;
  entry_time: number;
  exit_time: number;
  pnl_pct: number;
  size_usd?: number;
  side?: string;
};

export function holdHours(t: Pick<ClosedTradeLike, "entry_time" | "exit_time">): number {
  return Math.max(0, (t.exit_time - t.entry_time) / 3600);
}

/** Flat measured round-trip fee in percentage points of notional. */
export function measuredRtFeePct(_t?: ClosedTradeLike): number {
  return GMTRADE_RT_FEE_PCT;
}

/** Per-trade net PnL % after measured venue RT fee. */
export function netTradePnlPct(t: ClosedTradeLike): number {
  return t.pnl_pct - measuredRtFeePct(t);
}

/** Gross price PnL % (as stored by the trader). */
export function grossTradePnlPct(t: ClosedTradeLike): number {
  return t.pnl_pct;
}

export type PnlSeries = {
  /** Cumulative sum of per-trade net PnL % (unweighted). */
  totalNetPct: number;
  /** Cumulative sum of per-trade gross price PnL %. */
  totalGrossPct: number;
  /** Per-trade net series (same order as input). */
  netTrades: number[];
  /** Running cumulative net for sparkline (starts at 0). */
  cumulativeNet: number[];
  tradeCount: number;
  winRatePct: number | null;
};

export function summarizeTradePnl(trades: ClosedTradeLike[] | null | undefined): PnlSeries {
  if (!trades || trades.length === 0) {
    return {
      totalNetPct: 0,
      totalGrossPct: 0,
      netTrades: [],
      cumulativeNet: [],
      tradeCount: 0,
      winRatePct: null,
    };
  }

  const netTrades = trades.map(netTradePnlPct);
  const totalNetPct = netTrades.reduce((a, b) => a + b, 0);
  const totalGrossPct = trades.reduce((a, t) => a + t.pnl_pct, 0);
  const cumulativeNet: number[] = [0];
  let acc = 0;
  for (const n of netTrades) {
    acc += n;
    cumulativeNet.push(acc);
  }
  const wins = netTrades.filter((n) => n > 0).length;
  return {
    totalNetPct,
    totalGrossPct,
    netTrades,
    cumulativeNet,
    tradeCount: trades.length,
    winRatePct: (wins / netTrades.length) * 100,
  };
}

export function formatPnlPct(pct: number, digits = 2): string {
  const sign = pct > 0 ? "+" : "";
  return `${sign}${pct.toFixed(digits)}%`;
}
