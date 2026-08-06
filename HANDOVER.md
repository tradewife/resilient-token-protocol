# Handover for next agent — 2026-08-07

**Trader unblocked for real this time — two layers of bugs, both fixed.** The signal layer was fixed Aug 5 (below), but the trader stayed FLAT because a **second, independent blocker** appeared in the execution layer after Flash Trade's v2 API update. That is now fixed and verified with real mainnet trades on both sides.

## Layer 1 — Execution blocker (the one that kept the trader FLAT): fixed Aug 7

**Root cause:** after Flash's `feat/funded-pool-accounting` API build (Jul 30), every open built with `inputTokenSymbol: "SOL"` fails on-chain with **CustodyAmountLimit (6024 / 0x1788)** at `open_position_er.rs:245` — both sides, all sizes, all leverages. It was never pool capacity (Crypto.1 was at ~1% SOL utilization). The wrapper's size backoff (halving collateral below the 0.15 SOL floor) and the watchdog error loop were downstream symptoms, not the bug.

**Fix (commit `262f394`):** opens must use `inputTokenSymbol: "USDC"` with a USD-denominated `inputAmountUi` (`collateral_sol × SOL price`); the program draws collateral from the deposit ledger and converts internally.
- `executor.rs`: `open_position` is now **REST-first** using the USDC form (`open_position_via_rest`); the SDK wrapper is a last-resort fallback only (known-degraded). Do NOT revert ordering or collateral symbol.
- `cli/flash-sdk-wrapper.mjs`: fixed a ~13.5× unit bug — raw SOL lamports (9-decimals) were being passed as the USDC (6-decimal) collateral `amountIn`. Added `collateralToLockUnits()` conversion.
- Regression test `open_position_request_uses_v2_path_without_v2_prefix` now locks the USDC body shape.

**Verified on mainnet with real funds (Aug 7):** LONG open/close (`4Zk19z4...` / `4uHiyWq...`) and SHORT open/close (`61xLdve...` / `5n73xwz...`), both flat after close. Smoke script: `scripts/smoke-open-close-usdc.mjs` (re-runnable: `node scripts/smoke-open-close-usdc.mjs LONG|SHORT`). All 87 trader tests pass.

**Funding:** the deposit ledger holds ~1.5 SOL, wallet ~0.898 SOL, zero positions. No USDC is needed on the wallet — the program converts from the ledger internally.

## Layer 2 — Signal wiring bugs: fixed Aug 5 (commit `311457f`)

**Bug A+B:** 4h/1d candle buffers never refreshed after warmup, so slow-TF trends were computed from frozen candles. Fix: refresh 4h every 2h, 1d every 6h from Binance (`[REFRESH] 4h/1d: loaded N candles`).

**Bug C:** extra `bullish_count >= min_alignment` AND-gate on entry double-counted alignment (already baked into score via trend weight 0.4 × bull_count/3). Fix: gate on score only, matching the Python Survivor 2.69 reference.

## Config & params (unchanged, verified untampered)

`data/trader-strategy-config.json` — `min_alignment=2`, `signal_threshold=0.30`, tp=6.0, sl=2.5, trail=1.0, hold=96h, decay=48h, flip_delay=2h. **NO operator overrides active** (`RTP_TRADER_MIN_ALIGNMENT_OVERRIDE` / `RTP_TRADER_SIGNAL_THRESHOLD_OVERRIDE` UNSET). These are the WFA-validated values; do not tune them.

**Production wallet:** `HDQ79...` (NOT `Driyi...`). Trader service `40456d7a-5dfe-4112-8cf3-9a2ae5e3a910`.

**Next:** push commit `262f394`, redeploy rtp-trader, and watch Railway logs for a `[ENTRY] TX:` success on the first genuine ±0.30 score cross. The trader should fire ~once/day on LONG *and* SHORT signals.
