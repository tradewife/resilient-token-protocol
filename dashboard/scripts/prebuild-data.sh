#!/usr/bin/env bash
#
# Pre-build step: copy live data into dashboard/public/data/ for static export.
# Run before `next build` when using output: "export".
#
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
DATA_DIR="$REPO_ROOT/dashboard/public/data"
mkdir -p "$DATA_DIR"

python3 - "$REPO_ROOT" "$DATA_DIR" << 'PYEOF'
import json, os, pathlib, time, sys

REPO, DATA_DIR = sys.argv[1], sys.argv[2]
os.makedirs(DATA_DIR, exist_ok=True)

# ── cycle.json ──
cp = os.path.join(REPO, "data", "devnet-cycles", "latest", "cycle.json")
if os.path.isfile(cp):
    c = json.load(open(cp))
    pu = c.get("params_used", {})
    pn = c.get("params_next", {})
    diffs = []
    for k in sorted(pu.keys()):
        u, n = pu[k], pn.get(k, pu[k])
        if str(u) != str(n):
            diffs.append({"param": k, "from": u, "to": n})
    out = {
        "cycle_id": c.get("cycle_id"),
        "params_used": pu,
        "params_next": pn,
        "mutations_accepted": c.get("mutations_accepted", []),
        "mutations_rejected": c.get("mutations_rejected", []),
        "diffs": diffs,
        "used_llm": c.get("used_llm", False),
        "model_label": c.get("model_label", "?"),
        "memory_file_count": len(c.get("memory_files", [])),
        "timestamp": c.get("cycle_id"),
    }
    json.dump(out, open(os.path.join(DATA_DIR, "cycle.json"), "w"), indent=2)
    print("cycle.json written")
else:
    json.dump({"error": "cycle.json not found"}, open(os.path.join(DATA_DIR, "cycle.json"), "w"))
    print("cycle.json: fallback (no source)")

# ── memory.json ──
mem = os.path.join(REPO, "data", "swarm-memory")
files = []
if os.path.isdir(mem):
    files = [str(p) for p in pathlib.Path(mem).rglob("*") if p.is_file() and p.name != ".gitkeep"]
latest_file, latest_ts = "", 0
for f in files:
    try:
        ts = os.path.getmtime(f)
        if ts > latest_ts:
            latest_ts = ts
            latest_file = os.path.basename(f)
    except Exception:
        pass
breakdown = {}
for tier in ["core", "overview", "project", "working"]:
    d = os.path.join(mem, tier)
    if os.path.isdir(d):
        breakdown[tier] = sum(1 for p in pathlib.Path(d).rglob("*") if p.is_file() and p.name != ".gitkeep")
    else:
        breakdown[tier] = 0
json.dump({
    "fileCount": len(files),
    "latestFile": latest_file or None,
    "latestTimestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(latest_ts)) if latest_ts else None,
    "breakdown": breakdown,
}, open(os.path.join(DATA_DIR, "memory.json"), "w"), indent=2)
print(f"memory.json written ({len(files)} files)")

# ── night.json (latest night shift summary) ──
nr = os.path.join(REPO, "data", "night_results")
if os.path.isdir(nr):
    nights = sorted(d for d in os.listdir(nr) if os.path.isdir(os.path.join(nr, d)))
    if nights:
        latest = nights[-1]
        sf = os.path.join(nr, latest, "summary.json")
        lf = os.path.join(nr, latest, "leverage_optimization.json")
        rf = os.path.join(nr, latest, "report.md")
        lrf = os.path.join(nr, latest, "leverage_report.md")
        
        if os.path.isfile(lf):
            # Leverage optimization format
            lev = json.load(open(lf))
            lresults = lev.get("results", {})
            # Build night.json from leverage optimization data
            top_candidates = []
            for sym, results in lresults.items():
                sorted_r = sorted([r for r in results if not r.get("rejected")], 
                                  key=lambda r: r.get("calmar_ratio", 0), reverse=True)
                for r in sorted_r[:5]:
                    top_candidates.append({
                        "symbol": sym,
                        "params": r.get("params", {}),
                        "survivor_score": r.get("calmar_ratio", 0),
                        "oos_sharpe": r.get("oos_sharpe", 0),
                        "oos_consistency": r.get("consistency", 0),
                        "oos_max_dd": r.get("max_drawdown_pct", 0),
                        "overfitting_score": 0,
                        "fragility": 0,
                        "oos_avg_trades_per_fold": r.get("total_trades", 0) / max(lev.get("config", {}).get("wfa", {}).get("num_folds", 9), 1),
                        "rejected": r.get("rejected", False),
                    })
            
            # Market state from leverage report or defaults
            market_state = {}
            for sym in lev.get("config", {}).get("symbols", ["SOL/USDT"]):
                market_state[sym] = {
                    "current_adx": 0,
                    "current_regime": "TREND",
                    "volatility_percentile": 50,
                    "recent_30d_return_pct": 0,
                    "adx_trend": "STABLE",
                    "trend_pct": 50,
                }
            
            # Production baseline from top candidate
            prod_baseline = {}
            best = top_candidates[0] if top_candidates else None
            if best:
                prod_baseline[best["symbol"]] = {
                    "params": best["params"],
                    "survivor_score": best["survivor_score"],
                    "oos_sharpe": best["oos_sharpe"],
                    "oos_consistency": best["oos_consistency"],
                }
            
            summary = {
                "run_at": lev.get("run_at", ""),
                "runtime_seconds": lev.get("config", {}).get("runtime_seconds", 0),
                "num_folds": lev.get("config", {}).get("wfa", {}).get("num_folds", 9),
                "symbols": lev.get("config", {}).get("symbols", []),
                "market_state": market_state,
                "production_baseline": prod_baseline,
                "top_candidates": top_candidates,
                "_date": latest,
                "_report": open(lrf).read() if os.path.isfile(lrf) else 
                          (open(rf).read() if os.path.isfile(rf) else ""),
            }
            json.dump(summary, open(os.path.join(DATA_DIR, "night.json"), "w"), indent=2)
            print(f"night.json written from leverage_optimization.json ({latest})")
        elif os.path.isfile(sf):
            # Standard summary format
            summary = json.load(open(sf))
            summary["_date"] = latest
            summary["_report"] = open(rf).read() if os.path.isfile(rf) else ""
            json.dump(summary, open(os.path.join(DATA_DIR, "night.json"), "w"), indent=2)
            print(f"night.json written ({latest})")
        else:
            print("night.json: no summary.json or leverage_optimization.json in latest dir")
    else:
        print("night.json: no night_results subdirs")
else:
    print("night.json: night_results dir not found")

# ── trader-state.json (live autonomous trader state) ──
ts = os.path.join(REPO, "rtp", "swarm", "data", "trader-state.json")
if not os.path.isfile(ts):
    ts = os.path.join(REPO, "data", "trader-state.json")
if os.path.isfile(ts):
    import shutil
    shutil.copy2(ts, os.path.join(DATA_DIR, "trader-state.json"))
    print(f"trader-state.json written")
else:
    json.dump({"wallet": "", "open_position": None, "trade_history": [],
               "candle_count": 0, "last_poll": "", "total_pnl_sol": 0.0, "total_trades": 0},
              open(os.path.join(DATA_DIR, "trader-state.json"), "w"), indent=2)
    print("trader-state.json: fallback (no source)")
PYEOF

echo "Data files ready in $DATA_DIR"
