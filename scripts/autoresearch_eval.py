"""
Autoresearch evaluation step — outputs JSON metrics for the LLM to analyze.
"""
import os, sys, json
import numpy as np, pandas as pd
sys.stdout.reconfigure(line_buffering=True)
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
from scripts.per_symbol_optimizer import compute_indicators, simulate_trades, compute_metrics

DATA_DIR = os.path.join(os.path.dirname(__file__), "..", "data", "ohlcv")
SYMBOLS = ["BTC/USDT", "ETH/USDT", "SOL/USDT", "BNB/USDT"]

DEFAULT_CONFIGS = {
    "BTC/USDT": {"signal_threshold": 0.35, "min_alignment": 3, "take_profit_atr": 5.0, "stop_loss_atr": 1.5, "max_hold_hours": 72, "time_decay_hours": 24},
    "ETH/USDT": {"signal_threshold": 0.40, "min_alignment": 3, "take_profit_atr": 6.0, "stop_loss_atr": 2.5, "max_hold_hours": 96, "time_decay_hours": 48},
    "SOL/USDT": {"signal_threshold": 0.35, "min_alignment": 3, "take_profit_atr": 4.0, "stop_loss_atr": 1.5, "max_hold_hours": 48, "time_decay_hours": 24},
    "BNB/USDT": {"signal_threshold": 0.45, "min_alignment": 3, "take_profit_atr": 5.0, "stop_loss_atr": 1.5, "max_hold_hours": 96, "time_decay_hours": 48},
}

def full_eval(symbol, config):
    safe = symbol.replace("/", "_")
    df = pd.read_parquet(os.path.join(DATA_DIR, f"{safe}_1h.parquet"))
    df = compute_indicators(df)
    
    # Full sample
    trips = simulate_trades(df, config)
    full = compute_metrics(trips)
    
    # Rolling WFA windows
    window_size, step = 720, 168
    close = df["close"].values
    window_results = []
    for start in range(250, len(close) - window_size, step):
        wdf = df.iloc[start:start + window_size]
        wtrips = simulate_trades(wdf, config)
        if wtrips:
            wm = compute_metrics(wtrips)
            window_results.append({
                "period": f"{df.index[start].date()} to {df.index[start+window_size].date()}",
                "pnl": round(wm["total_pnl_pct"], 2),
                "wr": round(wm["win_rate"] * 100, 1),
                "pf": round(wm["pf"], 2),
                "sharpe": round(wm["sharpe"], 2),
                "trades": wm["round_trips"],
                "exits": wm.get("exit_reasons", {}),
            })
    
    # Per-window consistency
    profitable = sum(1 for w in window_results if w["pnl"] > 0)
    wfa_consistency = profitable / len(window_results) * 100 if window_results else 0
    wfa_mean_pnl = np.mean([w["pnl"] for w in window_results]) if window_results else 0
    wfa_mean_sharpe = np.mean([w["sharpe"] for w in window_results]) if window_results else 0
    
    # Worst windows (for analysis)
    window_results.sort(key=lambda x: x["pnl"])
    worst_windows = window_results[:3]
    best_windows = window_results[-3:]
    
    return {
        "symbol": symbol,
        "config": config,
        "full_sample": {
            "total_pnl": round(full["total_pnl_pct"], 2),
            "win_rate": round(full["win_rate"] * 100, 1),
            "pf": round(full["pf"], 2),
            "sharpe": round(full["sharpe"], 2),
            "max_dd": round(full["max_dd_pct"], 2),
            "round_trips": full["round_trips"],
            "avg_hold_hrs": round(full.get("avg_hold_hrs", 0), 1),
            "best_trade": round(full.get("best_trade", 0), 2),
            "worst_trade": round(full.get("worst_trade", 0), 2),
            "exit_reasons": full.get("exit_reasons", {}),
        },
        "wfa": {
            "windows": len(window_results),
            "profitable": profitable,
            "consistency_pct": round(wfa_consistency, 1),
            "mean_pnl_per_window": round(wfa_mean_pnl, 2),
            "mean_sharpe": round(wfa_mean_sharpe, 2),
            "worst_windows": worst_windows,
            "best_windows": best_windows,
        },
    }

if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument("--config-json", help="JSON file with per-symbol configs")
    parser.add_argument("--symbol", help="Evaluate single symbol")
    parser.add_argument("--params", help="Override params as JSON string")
    args = parser.parse_args()
    
    if args.config_json:
        with open(args.config_json) as f:
            configs = json.load(f)
    else:
        configs = DEFAULT_CONFIGS
    
    if args.params:
        override = json.loads(args.params)
        if args.symbol:
            configs[args.symbol] = override
    
    symbols = [args.symbol] if args.symbol else SYMBOLS
    
    results = {}
    for sym in symbols:
        print(f"Evaluating {sym}...", flush=True)
        results[sym] = full_eval(sym, configs[sym])
    
    print(json.dumps(results, indent=2, default=str))
