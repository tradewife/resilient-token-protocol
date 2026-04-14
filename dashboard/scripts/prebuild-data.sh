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
PYEOF

echo "Data files ready in $DATA_DIR"
