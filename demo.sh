#!/usr/bin/env bash
#
# RTP — Full Demo Script (Combined 3-Layer)
#
# Runs the complete narrative:
#   Layer 1: Python research engine validates a strategy (bridge-mode)
#   Layer 2: Rust swarm audits and approves the strategy assessment
#   Layer 3: On-chain treasury demonstrates fee flow and redistribution
#
# Usage: ./demo.sh
#
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$REPO_ROOT"

BOLD='\033[1m'
DIM='\033[2m'
GREEN='\033[32m'
CYAN='\033[36m'
YELLOW='\033[33m'
RED='\033[31m'
RESET='\033[0m'

# Variables set by Layer 1, consumed by Layer 3
PROJECTED_YIELD=""
BRIDGE_CONFIDENCE=""
STRATEGY="${STRATEGY:-unknown}"
YIELD="${YIELD:-0.0%}"
FOLDS="${FOLDS:-0}"

banner() {
  echo ""
  echo -e "${CYAN}══════════════════════════════════════════════════════════════${RESET}"
  echo -e "${CYAN}  $1${RESET}"
  echo -e "${CYAN}══════════════════════════════════════════════════════════════${RESET}"
}

step() {
  echo ""
  echo -e "${BOLD}▸ $1${RESET}"
  echo -e "${DIM}──────────────────────────────────────────────────${RESET}"
}

ok() {
  echo -e "  ${GREEN}✅ $1${RESET}"
}

info() {
  echo -e "  ${CYAN}→ $1${RESET}"
}

note() {
  echo -e "  ${DIM}$1${RESET}"
}

# ─── Setup ──────────────────────────────────────────────────────────────

banner "RTP — Resilient Token Protocol: Full Demo"

echo ""
echo "  Any Solana token adopts RTP → fees route to the swarm →"
echo "  swarm researches and validates yield strategies →"
echo "  projected yield informs on-chain treasury distribution."
echo ""

# Check prerequisites
step "Prerequisites"

if [ -f ".venv/bin/activate" ]; then
  source .venv/bin/activate
  ok "Python venv active"
else
  echo -e "  ${RED}ERROR: .venv not found. Run: python -m venv .venv && pip install pandas numpy ccxt pyarrow redis${RESET}"
  exit 1
fi

if command -v cargo &>/dev/null; then
  ok "Rust toolchain: $(rustc --version 2>/dev/null | head -1)"
else
  echo -e "  ${RED}ERROR: cargo not found${RESET}"
  exit 1
fi

if command -v node &>/dev/null; then
  ok "Node.js: $(node --version)"
else
  echo -e "  ${RED}ERROR: node not found — required for Layer 3 on-chain demo${RESET}"
  exit 1
fi

if [ ! -d "rtp/programs/rtp-treasury/node_modules" ]; then
  info "Installing treasury npm deps..."
  (cd rtp/programs/rtp-treasury && npm ci --quiet 2>/dev/null) || {
    echo -e "  ${YELLOW}⚠ npm ci failed — Layer 3 may encounter errors${RESET}"
  }
fi

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# LAYER 1: PYTHON RESEARCH — Strategy Validation
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

banner "LAYER 1: Python Research Engine (Yield Brain)"

# ── Paper Trader Status ────────────────────────────────────────────────

step "Paper Trader — Live Market Validation"
if [ -f "data/paper_trading/state.json" ]; then
  PAPER_INFO=$(python3 -c "
import json, sys
try:
    s = json.load(open('data/paper_trading/state.json'))
    start = s.get('start_time','unknown')[:10]
    n_trades = len(s.get('round_trips', []))
    n_signals = len(s.get('signals', []))
    balance = s.get('balance', 10000)
    pnl = ((balance - 10000) / 10000) * 100
    pos = 'none'
    positions = s.get('positions', {})
    if positions:
        pos = ', '.join(f'{k}: {v.get(\"side\",\"?\")}'for k,v in positions.items())
    print(f'Live since {start} | {n_signals} signals evaluated, {n_trades} round-trip trades | PnL: {pnl:+.1f}% | Position: {pos}')
except Exception as e:
    print(f'Error reading state: {e}')
" 2>/dev/null)
  ok "Paper trader (real Binance data): $PAPER_INFO"
else
  note "  Paper trader state not yet populated (runs nightly via CI)"
fi

# ── Bridge Round-Trip ──────────────────────────────────────────────────

step "WFA Strategy Assessment — Python → Rust Bridge"
echo "  The Rust swarm calls the Python research binary via bridge.rs."
echo "  Python evaluates the strategy on real OHLCV data using 9-fold"
echo "  walk-forward analysis and returns the out-of-sample performance."
echo ""

if [ -f "night_shift.bin" ]; then
  BRIDGE_RESPONSE=$(echo '{"symbol":"SOL/USDT","config":{"params":{"signal_threshold":0.40}}}' | \
    ./night_shift.bin --bridge-mode 2>/dev/null)

  if [ -n "$BRIDGE_RESPONSE" ]; then
    STRATEGY=$(echo "$BRIDGE_RESPONSE" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('strategy','?'))")
    YIELD=$(echo "$BRIDGE_RESPONSE" | python3 -c "import sys,json; d=json.load(sys.stdin); print(f\"{d.get('yield_estimate',0):.1f}%\")")
    CONFIDENCE=$(echo "$BRIDGE_RESPONSE" | python3 -c "import sys,json; d=json.load(sys.stdin); print(f\"{d.get('confidence',0):.2f}\")")
    FOLDS=$(echo "$BRIDGE_RESPONSE" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('folds_validated',0))")
    CONSISTENCY=$(echo "$BRIDGE_RESPONSE" | python3 -c "import sys,json; d=json.load(sys.stdin); print(f\"{d.get('consistency',0):.2f}\")")

    # Store for Layer 3 handoff
    PROJECTED_YIELD=$(echo "$BRIDGE_RESPONSE" | python3 -c "import sys,json; d=json.load(sys.stdin); print(f\"{d.get('yield_estimate',0):.2f}\")")
    BRIDGE_CONFIDENCE="$CONFIDENCE"

    ok "Strategy assessment complete (source: WFA backtest)"
    info "Strategy:      $STRATEGY"
    info "Projected OOS: +$YIELD annual (not realized — walk-forward estimate)"
    info "Confidence:    $CONFIDENCE"
    info "WFA folds:     $FOLDS validated"
    info "Consistency:   $CONSISTENCY"
    echo ""
    note "  Full assessment response:"
    echo "$BRIDGE_RESPONSE" | python3 -m json.tool 2>/dev/null | sed 's/^/    /'
  else
    echo -e "  ${YELLOW}⚠ Bridge binary returned empty (running without data)${RESET}"
    note "  Falling back to latest devnet cycle for strategy data"
  fi
else
  echo -e "  ${YELLOW}⚠ night_shift.bin not found — skipping bridge assessment${RESET}"
  note "  Build with: cd rtp/swarm && cargo test bridge::real_binary_bridge_mode_integration"
fi

# Fallback: read strategy data from latest devnet cycle if bridge didn't run
if [ -z "$PROJECTED_YIELD" ] && [ -f "data/devnet-cycles/latest/cycle.json" ]; then
  CYCLE_STRATEGY=$(python3 -c "
import json
c = json.load(open('data/devnet-cycles/latest/cycle.json'))
p = c.get('params_used', {})
print(f\"SOL/USDT (signal_threshold={p.get('signal_threshold','?')}, tp_atr={p.get('tp_atr','?')})\")" 2>/dev/null || echo "unknown")
  STRATEGY="$CYCLE_STRATEGY"
  info "Strategy from latest devnet cycle: $CYCLE_STRATEGY"
fi

# ── Autonomous Devnet Cycles ──────────────────────────────────────────

step "Autonomous Devnet Cycles (6h cron via GitHub Actions)"
CYCLE_COUNT=$(ls -d data/devnet-cycles/20* 2>/dev/null | wc -l || echo "0")
if [ "$CYCLE_COUNT" -gt 0 ] 2>/dev/null; then
  ok "$CYCLE_COUNT autonomous cycles completed"
  CYCLE_SUMMARY=$(python3 -c "
import json
c = json.load(open('data/devnet-cycles/latest/cycle.json'))
n_acc = len(c.get('mutations_accepted', []))
n_rej = len(c.get('mutations_rejected', []))
llm = c.get('used_llm', False)
model = c.get('model_label', '?')
print(f'{n_acc} mutations accepted, {n_rej} rejected | LLM: {\"yes\" if llm else \"no\"} ({model})')
" 2>/dev/null || echo "cycle data unavailable")
  info "Latest cycle: $CYCLE_SUMMARY"
else
  note "No committed devnet cycles found (daemon runs every 6h via CI)"
fi

# ── Strategy Adaptation Diff ──────────────────────────────────────────

step "Strategy Adaptation (latest cycle)"
if [ -f "data/devnet-cycles/latest/cycle.json" ]; then
  python3 -c "
import json
c = json.load(open('data/devnet-cycles/latest/cycle.json'))
used = c.get('params_used', {})
next_p = c.get('params_next', {})
changed = False
for k in sorted(used.keys()):
    u, n = used[k], next_p.get(k, used[k])
    arrow = '→' if str(u) != str(n) else '='
    if arrow == '→':
        changed = True
    print(f'    {k}: {u} {arrow} {n}')
if not changed:
    print('    (no parameter changes in latest cycle)')
" 2>/dev/null || note "Could not parse cycle data"
else
  note "No cycle data available for adaptation diff"
fi

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# LAYER 2: RUST SWARM — Audit, Approve, Assess
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

banner "LAYER 2: Rust Swarm Runtime (Coordinator + 6 Wings)"

step "Swarm Demo Loop — Propose → Soulguard → Audit → Assess"
echo "  The Trading Wing proposes a strategy deployment. The Coordinator"
echo "  routes through soulguard (soulcontract check) → Audit Wing (3-agent"
echo "  tribunal) → sends ExecutePermit → Trading validates via bridge."
echo ""

cargo run --bin rtp-demo --manifest-path rtp/swarm/Cargo.toml 2>/dev/null || {
  echo -e "  ${YELLOW}⚠ Demo binary not built — building...${RESET}"
  cargo build --bin rtp-demo --manifest-path rtp/swarm/Cargo.toml 2>&1 | tail -3
  cargo run --bin rtp-demo --manifest-path rtp/swarm/Cargo.toml 2>/dev/null
}

TEST_COUNT=$(cargo test --manifest-path rtp/swarm/Cargo.toml 2>/dev/null | grep "test result:" | head -1 | grep -oP '\d+(?= passed)' || echo "unknown")
echo ""
if [ "$TEST_COUNT" = "unknown" ]; then
  note "Swarm runtime: test count unavailable (cargo test parse failed)"
else
  ok "Swarm runtime: $TEST_COUNT tests passing"
fi
info "6 wings functional (Trading, Security, Evolve, Knowledge, Audit, Futureproof)"
info "Multi-stage quality gate: soulguard → router → audit tribunal"

# ── On-chain constraint rejection proof ─────────────────────────────────
echo ""
step "Constraint Rejection (on-chain proof)"
echo "  The Anchor program enforces hard constraints that cannot be bypassed."
echo "  evolve_phase rejects when treasury balance < 50B tokens (BelowThreshold)."
echo "  The rtp-demo replays the actual rejection from the deployed devnet program."
echo ""
note "  Proof: Anchor test suite in rtp/programs/rtp-treasury/tests/treasury.ts"
note "  evolve_phase BelowThreshold test (line 777)"
note "  Redistribution tx (70/20/10 enforced):"
note "    https://explorer.solana.com/tx/9HzWgBfwYxs5ModdjF5mT6gdTfayQq8mMYipopyHfGPmYqk6KESHFqgDrc9Mcie573ttcdPqMHSyJP5nNBKK3bR?cluster=devnet"

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# LAYER 3: ON-CHAIN TREASURY — Fee Flow + Redistribution
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

banner "LAYER 3: On-Chain Treasury (Solana / Anchor)"

step "Treasury Program — Fee Flow + Redistribution"
echo "  The treasury program runs on Solana devnet. It demonstrates:"
echo "    1. Token adopts RTP (TransferFeeConfig enabled)"
echo "    2. Trading fees auto-route to Treasury PDA"
echo "    3. Threshold hit → check_redistribute (70/20/10 split)"
echo "    4. Swarm validates strategy → treasury approves → distribute"
echo "    5. Self-hydration (hydrate_swarm with runway check)"
echo ""

# Pass strategy assessment from Layer 1 → Layer 3
if [ -n "$PROJECTED_YIELD" ]; then
  info "Handoff from Layer 1: projected +${PROJECTED_YIELD}% OOS yield (confidence: $BRIDGE_CONFIDENCE)"
  export PROJECTED_YIELD
  export BRIDGE_CONFIDENCE
fi

# Check if program is built
if [ -f "rtp/programs/rtp-treasury/target/types/rtp_treasury.ts" ] || \
   [ -f "rtp/programs/rtp-treasury/target/idl/rtp_treasury.json" ]; then
  ok "Treasury program built (Anchor 1.0)"
else
  echo -e "  ${YELLOW}⚠ Treasury not built — run: cd rtp/programs/rtp-treasury && anchor build${RESET}"
fi

# Program liveness check on devnet
step "Program Liveness Check"
PROGRAM_INFO=$(curl -s https://api.devnet.solana.com -X POST -H "Content-Type: application/json" \
  -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getAccountInfo\",\"params\":[\"4LvsHbe9LLwgogcDbH7ieTsGcWZctjYFZkzZwaHDM8Ad\",{\"encoding\":\"base64\"}]}" 2>/dev/null)
if echo "$PROGRAM_INFO" | python3 -c "import sys,json; d=json.load(sys.stdin); assert d.get('result',{}).get('value') is not None" 2>/dev/null; then
  ok "Program 4LvsHb...M8Ad is live on devnet"
else
  echo -e "  ${RED}━━━ BLOCKER: Program GC'd from devnet ━━━${RESET}"
  echo -e "  ${RED}Cannot present without a live program.${RESET}"
  echo -e "  ${RED}Re-deploy now:${RESET}"
  echo -e "  ${BOLD}  cd rtp/programs/rtp-treasury && anchor deploy --provider.cluster devnet${RESET}"
  echo -e "  ${RED}Then update PROGRAM_ID in page.tsx and demo.sh if it changed.${RESET}"
  exit 1
fi

# Check if local validator or devnet is reachable
step "On-Chain Demo Execution"
if curl -s http://localhost:8899 -X POST -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' 2>/dev/null | grep -q "ok"; then
  info "Local validator running — executing on-chain demo..."
  echo ""
  cd rtp/programs/rtp-treasury && npx tsx scripts/devnet-demo.ts 2>&1 || echo -e "  ${YELLOW}⚠ On-chain demo encountered errors (see above)${RESET}"; cd "$REPO_ROOT"
elif curl -s https://api.devnet.solana.com -X POST -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' 2>/dev/null | grep -q "ok"; then
  info "Devnet reachable — executing on-chain demo..."
  echo ""
  cd rtp/programs/rtp-treasury && ANCHOR_PROVIDER_URL=https://api.devnet.solana.com npx tsx scripts/devnet-demo.ts 2>&1 || echo -e "  ${YELLOW}⚠ On-chain demo encountered errors (see above)${RESET}"; cd "$REPO_ROOT"
else
  echo -e "  ${RED}LAYER 3: Cannot reach devnet or local validator. On-chain demo skipped.${RESET}"
  echo ""
  note "  To run the on-chain demo:"
  note "    cd rtp/programs/rtp-treasury && npm run demo:devnet"
  echo ""
  note "  On-chain constraint rejection proof (already verified in Anchor tests):"
  note "    rtp/programs/rtp-treasury/tests/treasury.ts — evolve_phase BelowThreshold"
fi

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# Invariants + Summary
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

banner "Architecture Summary"

echo ""
echo "  ┌──────────────────────────────────────────────────────────┐"
echo "  │                  RTP — THREE-LAYER STACK                 │"
echo "  ├──────────────────────────────────────────────────────────┤"
echo "  │  ON-CHAIN (Solana)                                       │"
echo "  │  Treasury PDA: fees → assess → redistribute → self-hydrate│"
echo "  ├──────────────────────────────────────────────────────────┤"
echo "  │  SWARM RUNTIME (Rust)                                    │"
echo "  │  Coordinator → 6 wings (Trading, Security, Evolve,       │"
echo "  │  Knowledge, Audit, Futureproof) → $TEST_COUNT tests       │"
echo "  ├──────────────────────────────────────────────────────────┤"
echo "  │  RESEARCH LAYER (Python)                                 │"
echo "  │  30K configs/night → 9-fold WFA → full-sim validation    │"
echo "  │  Assessment: $STRATEGY, +$YIELD OOS, $FOLDS folds         │"
echo "  └──────────────────────────────────────────────────────────┘"
echo ""
echo "  Invariants (enforced on-chain):"
echo "    ✅ PDA owns treasury (no private key risk)"
echo "    ✅ TransferFeeConfig immutable (withdraw authority = PDA)"
echo "    ✅ CPI-only transfers (atomic, verifiable)"
echo "    ✅ Agent proposes, human approves irreversible actions"
echo "    ✅ No SOL liquidation (USDC-only yield flows)"
echo "    ✅ Phase transitions irreversible + threshold enforced"
echo "    ✅ Soulcontract amendments require human signature"
echo "    ✅ Auto-rollback if performance degrades > 5%"
echo "    ✅ Self-hydration only if > 90-day runway"
echo ""
echo -e "${CYAN}══════════════════════════════════════════════════════════════${RESET}"
echo -e "${BOLD}  Demo complete.${RESET}"
echo -e "${CYAN}══════════════════════════════════════════════════════════════${RESET}"
echo ""
echo -e "  ${DIM}Run timestamp: $(date -u +"%Y-%m-%dT%H:%M:%SZ")${RESET}"
echo -e "  ${DIM}Cycle count:   $CYCLE_COUNT autonomous cycles${RESET}"
if [ -f "data/paper_trading/state.json" ] && [ -n "$PAPER_INFO" ]; then
  echo -e "  ${DIM}Paper trader:  $PAPER_INFO${RESET}"
else
  echo -e "  ${DIM}Paper trader:  no state (not yet populated)${RESET}"
fi
