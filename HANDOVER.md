# Handover for next agent — 2026-08-05

**Trader unblocked — real root cause found and fixed.** The trader was blocked not by `min_alignment` semantics alone, but by **three bugs in the wiring** that together pinned the score below the 0.30 threshold and froze the slow-TF trends. Commit `311457f` (deploy `c6ff1910`) fixes all three:

**Bug A+B: 4h/1d candle buffers never refreshed after warmup.**
`run_cycle` only called `buffer_1h.append_tick()`. The 4h and 1d `CandleBuffer`s never received fresh data, so `tf_4h.trend` and `tf_1d.trend` were computed from Binance candles frozen at deploy time. After 24h the 1d trend compared a >1-day-old close against a stale SMA — bullish/bearish counts could never flip with the market. **Fix:** refresh 4h every 2h, 1d every 6h from Binance via `last_4h_refresh`/`last_1d_refresh` timestamps passed into `run_cycle`. Logs show `[REFRESH] 4h/1d: loaded N candles`.

**Bug C: extra `bullish_count >= min_alignment` AND-gate on entry.**
The Rust entry had `score > threshold && bullish_count >= min_alignment`, but the Python Survivor 2.69 reference (`run_backtest_r2.py` line ~257) only gates on `if score > threshold`. The alignment count is already baked into the score via the trend weight (0.4 × bull_count/3), so the extra gate double-counted it. With `min_alignment=2`, trend alone gives 0.267 — just under 0.30 — and in a sideways market momentum/MR/BB don't fire, capping the score at 0.267 forever. **Fix:** gate on score only, matching Python.

**Verified live (2026-08-05 ~19:00 UTC):** score now varies with the market (0.057 → 0.117 → varies with RSI/BB) instead of being pinned at ±0.267. Reason sets change in real time (RSI crossing 69→58, `rsi_near_overbought_daily_bear` appearing/disappearing), confirming the 4h/1d buffers are live.

**Active config:** `data/trader-strategy-config.json` — `min_alignment=2` (matches Python default), `signal_threshold=0.30`, tp=6.0, sl=2.5, trail=1.0, hold=96h, decay=48h, flip_delay=2h. **NO operator overrides active** (`RTP_TRADER_MIN_ALIGNMENT_OVERRIDE` / `RTP_TRADER_SIGNAL_THRESHOLD_OVERRIDE` UNSET — verified via `override show`).

**Production wallet:** `HDQ79...` (NOT `Driyi...`). Service `40456d7a-5dfe-4112-8cf3-9a2ae5e3a910`. All 87 trader tests pass.

**Next:** monitor the next real entry/exit event to confirm the SDK close path (arg order + SendErResult extraction from `e9596b1`) works end-to-end in production. The trader should now fire on genuine LONG *and* SHORT signals when the score crosses ±0.30.
