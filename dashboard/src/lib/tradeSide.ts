/** Minimal trade fields needed to infer long vs short from stored PnL math. */
export type TradeSideInput = {
  entry_price: number;
  exit_price: number;
  pnl_pct: number;
  side?: string;
};

export type TradeSide = "Long" | "Short";

/** Match rtp-trader TradeRecord::infer_side_from_pnl (long/short PnL vs stored pnl_pct). */
export function inferTradeSide(trade: TradeSideInput): TradeSide {
  const entry = trade.entry_price;
  if (entry <= 0) return "Long";
  const longPnl = ((trade.exit_price - entry) / entry) * 100;
  const shortPnl = ((entry - trade.exit_price) / entry) * 100;
  const longErr = Math.abs(longPnl - trade.pnl_pct);
  const shortErr = Math.abs(shortPnl - trade.pnl_pct);
  return shortErr < longErr ? "Short" : "Long";
}

export function tradeSideCssClass(side: TradeSide): "long" | "short" {
  return side === "Short" ? "short" : "long";
}