"""
LLM Strategy Selector — reads strategy library, night shift results, and
dead ends to select the most promising strategies for the next exploration run.

Uses OpenAI-compatible API (same env vars as Rust Evolve Wing).
Falls back to round-robin through untested strategies if LLM unavailable.

Usage:
  python -m research.orchestration.llm_strategy_selector
  python -m research.orchestration.llm_strategy_selector --fallback
"""
import argparse
import json
import os
import sys
from datetime import datetime, timezone
from typing import Dict, List, Optional

import numpy as np

sys.stdout.reconfigure(line_buffering=True)
sys.stderr.reconfigure(line_buffering=True)

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))

from research.strategy_plugins import PLUGINS

ROOT = os.path.join(os.path.dirname(__file__), "..", "..")
LIBRARY_PATH = os.path.join(ROOT, "research", "strategy_library.md")
DEAD_ENDS_PATH = os.path.join(ROOT, "research", "dead_ends.md")
RESULTS_DIR = os.path.join(ROOT, "data", "night_results")

# Strategy IDs available as plugins
AVAILABLE_STRATEGIES = list(PLUGINS.keys())


def log(msg: str):
    ts = datetime.now(timezone.utc).strftime("%H:%M:%S")
    print(f"[{ts}] {msg}", flush=True)


def _load_strategy_library() -> str:
    """Load strategy library markdown."""
    if os.path.exists(LIBRARY_PATH):
        with open(LIBRARY_PATH) as f:
            return f.read()
    return ""


def _load_dead_ends() -> str:
    """Load dead ends log."""
    if os.path.exists(DEAD_ENDS_PATH):
        with open(DEAD_ENDS_PATH) as f:
            return f.read()
    return ""


def _load_latest_results() -> str:
    """Load the most recent night shift report."""
    if not os.path.exists(RESULTS_DIR):
        return ""

    # Find most recent date directory
    date_dirs = sorted([
        d for d in os.listdir(RESULTS_DIR)
        if os.path.isdir(os.path.join(RESULTS_DIR, d))
    ], reverse=True)

    if not date_dirs:
        return ""

    latest = os.path.join(RESULTS_DIR, date_dirs[0])
    # Prefer leverage report, then regular report
    for name in ["leverage_report.md", "report.md"]:
        path = os.path.join(latest, name)
        if os.path.exists(path):
            with open(path) as f:
                content = f.read()
            return f"Latest report ({date_dirs[0]}/{name}):\n{content[:3000]}"

    return ""


def _load_latest_results_json() -> Dict:
    """Load the most recent results JSON."""
    if not os.path.exists(RESULTS_DIR):
        return {}

    date_dirs = sorted([
        d for d in os.listdir(RESULTS_DIR)
        if os.path.isdir(os.path.join(RESULTS_DIR, d))
    ], reverse=True)

    if not date_dirs:
        return {}

    latest = os.path.join(RESULTS_DIR, date_dirs[0])
    for name in ["leverage_optimization.json", "full_results.json"]:
        path = os.path.join(latest, name)
        if os.path.exists(path):
            with open(path) as f:
                return json.load(f)

    return {}


def select_with_llm(n_select: int = 3) -> List[Dict]:
    """
    Use LLM to select the most promising strategies from the library.

    Returns a list of strategy specs:
    [
        {"id": "S02", "reason": "...", "priority": 1},
        ...
    ]
    """
    import urllib.request
    import urllib.error

    api_base = os.environ.get("LLM_API_BASE_URL", "https://api.openai.com/v1")
    api_key = os.environ.get("LLM_API_KEY", "")
    model = os.environ.get("LLM_MODEL", "gpt-4o-mini")

    if not api_key:
        log("No LLM_API_KEY set, falling back to round-robin")
        return select_fallback(n_select)

    library = _load_strategy_library()
    dead_ends = _load_dead_ends()
    latest_report = _load_latest_results()

    prompt = f"""You are a quantitative trading strategist for crypto perpetuals on Solana.
Your task: select the {n_select} most promising strategies from the library below
that should be tested in tonight's exploration run.

CURRENT STATE:
- Best strategy: SOL/USDT Survivor 2.69 (MultiTF trend-following, Calmar=44.89 at 9x)
- Current config: signal_threshold=0.25, tp=5.0, sl=2.7, trail=0.14, align=3, leverage=9x
- Execution venue: Flash Trade on-chain perps (SOL/USDT only currently)
- We can only use OHLCV data (no order book, no funding rate, no on-chain)

AVAILABLE PLUGINS (already implemented, ready to test):
{json.dumps(AVAILABLE_STRATEGIES)}

STRATEGY LIBRARY:
{library[:6000]}

DEAD ENDS (strategies/approaches that failed):
{dead_ends[:2000]}

{latest_report[:2000]}

Select exactly {n_select} strategies. For each, provide:
1. Strategy ID (must be one of: {AVAILABLE_STRATEGIES})
2. Reason (1-2 sentences why this is promising NOW)
3. Priority (1=highest, 2=medium, 3=lower)

Focus on strategies that COMPLEMENT the current trend-following approach —
either work in different regimes (ranging), capture different edges (mean reversion,
volatility breakout), or have structural alpha in crypto (squeeze patterns).

Respond in JSON format only:
{{"selections": [{{"id": "S02", "reason": "...", "priority": 1}}, ...]}}"""

    url = f"{api_base}/chat/completions"
    payload = json.dumps({
        "model": model,
        "messages": [
            {"role": "system", "content": "You are a quantitative trading strategist. Respond only in valid JSON."},
            {"role": "user", "content": prompt},
        ],
        "temperature": 0.3,
        "max_tokens": 1000,
    }).encode()

    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {api_key}",
    }

    try:
        req = urllib.request.Request(url, data=payload, headers=headers, method="POST")
        with urllib.request.urlopen(req, timeout=30) as resp:
            data = json.loads(resp.read().decode())
            content = data["choices"][0]["message"]["content"]

            # Parse JSON from response (handle markdown code fences)
            content = content.strip()
            if content.startswith("```"):
                content = content.split("\n", 1)[1] if "\n" in content else content[3:]
                if content.endswith("```"):
                    content = content[:-3]
                content = content.strip()

            result = json.loads(content)
            selections = result.get("selections", [])

            # Validate
            valid = []
            for s in selections:
                sid = s.get("id", "")
                if sid in AVAILABLE_STRATEGIES:
                    valid.append(s)

            if not valid:
                log("LLM returned no valid selections, falling back")
                return select_fallback(n_select)

            log(f"LLM selected {len(valid)} strategies:")
            for s in valid:
                log(f"  {s['id']}: {s.get('reason', 'no reason')} (priority={s.get('priority', 3)})")

            return valid[:n_select]

    except (urllib.error.URLError, json.JSONDecodeError, KeyError, TimeoutError) as e:
        log(f"LLM call failed ({e}), falling back to round-robin")
        return select_fallback(n_select)


def select_fallback(n_select: int = 3) -> List[Dict]:
    """
    Fallback: round-robin through untested strategies.

    Priority order from strategy_library.md classification.
    """
    # Priority order: S02 > S04 > S06 > S13 > S10
    priority_order = ["S02", "S04", "S06", "S13", "S10"]

    selections = []
    for sid in priority_order[:n_select]:
        plugin = PLUGINS[sid]
        selections.append({
            "id": sid,
            "reason": f"Round-robin selection (LLM unavailable). {plugin.description}",
            "priority": len(selections) + 1,
        })

    log(f"Fallback selected: {[s['id'] for s in selections]}")
    return selections


def select_strategies(use_llm: bool = True, n_select: int = 3) -> List[Dict]:
    """Main entry: select strategies for tonight's exploration run."""
    log(f"Selecting {n_select} strategies for exploration "
        f"(llm={'on' if use_llm else 'off'})")

    if use_llm:
        return select_with_llm(n_select)
    return select_fallback(n_select)


def main():
    parser = argparse.ArgumentParser(description="LLM Strategy Selector")
    parser.add_argument("--fallback", action="store_true",
                        help="Skip LLM, use round-robin fallback")
    parser.add_argument("--n", type=int, default=3,
                        help="Number of strategies to select")
    parser.add_argument("--json", action="store_true",
                        help="Output as JSON only")
    args = parser.parse_args()

    use_llm = not args.fallback
    selections = select_strategies(use_llm=use_llm, n_select=args.n)

    if args.json:
        print(json.dumps(selections, indent=2))
    else:
        for s in selections:
            plugin = PLUGINS.get(s["id"])
            grid_size = 1
            if plugin:
                grid = plugin().param_grid()
                grid_size = 1
                for v in grid.values():
                    grid_size *= len(v)
            log(f"\n{s['id']}: {s.get('reason', '')}")
            log(f"  Grid size: {grid_size} candidates")


if __name__ == "__main__":
    main()
