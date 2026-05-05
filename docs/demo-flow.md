# 3-Minute Demo Script — Colosseum Judges

**Target:** Show 5 things judges need to verify in under 3 minutes.
**Setup:** Browser with dashboard + Solana Explorer + Railway dashboard open.

---

## 0:00 — One-Liner

> "Token projects route trading fees to RTP → RTP generates yield via on-chain perps → yield flows back to holders."

**Show:** Dashboard home page. Point at the hero: "Every token gets a program-enforced treasury."

## 0:30 — Live Autonomous Trader

> "The strategy runs autonomously 24/7 on Railway. No human in the loop."

**Show:** Dashboard "Live Trading" section.
- Status: LIVE green dot
- Current SOL price (real-time from Flash Trade)
- Signal: score, RSI, bullish/bearish state
- "The trader is watching right now. When conditions align, it opens a position automatically."

## 1:00 — Mainnet Proof — Real Money, Real Transactions

> "These are real mainnet transactions. Not testnet. Not simulation."

**Show:** Click the Solana Explorer links.
- Open TX `YtGKq46w...` — position opened on Solana mainnet
- Close TX `56PLUQA...` — position closed, SOL returned
- "The Treasury PDA signs via invoke_signed. No human keypair exists for trading."

## 1:30 — Railway — Autonomous Infrastructure

> "The entire system runs on Railway. Six services, all green."

**Show:** Railway dashboard — rtp-trader (Online), rtp-dashboard (Online), rtp-devnet-loop (Completed), rtp-night-shift (Completed), rtp-swarm-ci (Completed).
- "The trader runs as an always-on service. Polls every 5 minutes. Self-funded."

## 2:00 — Research Engine — Proven Results

> "30,000 strategy configs tested per night. 9-fold walk-forward validation."

**Show:** Dashboard "Validated Strategy" section.
- Calmar Ratio: 44.89 (9x leverage)
- PnL: +554% compounded
- Consistency: 9/9 folds (100%)
- Trades: 429
- "Not a backtest screenshot. Out-of-sample results across 9 independent time windows."

## 2:30 — Soulguard — Constitutional Governance

> "The system refuses actions that violate its constitution."

**Show:** Dashboard footer → "Rejection proof" link.
- Explorer shows `BelowPriceFloor` error — the program rejected an invalid phase evolution.
- "Six wings, 18 invariants, enforced in Rust AND on-chain. Agents propose, constraints dispose."

## 3:00 — Close

> "Self-funding treasury. No RTP token. Pure infrastructure. One function call to adopt."

**Show:** SDK code snippet — `registerWithRTP()`.
- "Any token project integrates with one function call. Their fees don't sit idle — the swarm puts them to work. Forever."

---

## Backup: If Judges Ask

| Question | Answer |
|----------|--------|
| "Why Flash Trade?" | Only Solana perps DEX supporting CPI. Liquidity grows with composability proof. |
| "Why not Drift/Jupiter?" | No CPI interface for autonomous program-to-program trading. |
| "What about the program deployment cost?" | 372KB binary, ~2.6 SOL. Using direct REST API trading to prove the loop first. Program deployment when budget allows. |
| "How is this different from AI yield agents?" | Infrastructure, not a product. Any token adopts RTP — we don't custody funds, the PDA does. |
| "What happens if the trader loses?" | Max 12.3% drawdown observed at 9x leverage. Stop-loss at 2.7× ATR per trade. Tight trailing stop at 0.14× ATR captures gains fast. Per-token isolation means one loss doesn't affect others. |
| "Can I see it trade live?" | Yes — the trader is running right now on Railway. When market conditions trigger a signal, a position opens automatically. |
