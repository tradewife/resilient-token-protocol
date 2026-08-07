#!/usr/bin/env python3
"""Fetch a second year of 20m OHLCV (Aug 2024 -> Aug 2025) for S15 gap closure.

The existing SOL_USDT_20m.parquet covers 2025-08-05 -> 2026-08-05 (1 year)
and is 5m data resampled to 20m boundaries (Binance has no native 20m TF).
This fetches 5m for 2024-08-06 -> 2025-08-05 and resamples the same way.
The v5 verdict's weakest fold had only 3 trades (floor: 10/fold); a second
year of data lets the 9-fold WFA work with ~2x thickness. Writes
SOL/BTC/ETH_USDT_20m_y2.parquet; the gap-closure script concatenates them.
"""
import asyncio
import os
from datetime import datetime

import ccxt.async_support as ccxt
import pandas as pd

OUT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "data", "ohlcv"))
SYMBOLS = ["SOL/USDT", "BTC/USDT", "ETH/USDT"]
SRC_TF = "5m"
RESAMPLE_MIN = 20
START = datetime(2024, 8, 6)
END = datetime(2025, 8, 5, 20, 20)  # matches existing window start


def resample_20m(df5: pd.DataFrame) -> pd.DataFrame:
    """5m -> 20m OHLCV on 20m boundaries (matches existing window build)."""
    rule = f"{RESAMPLE_MIN}min"
    agg = {
        "open": "first",
        "high": "max",
        "low": "min",
        "close": "last",
        "volume": "sum",
    }
    out = df5.resample(rule, label="left", closed="left").agg(agg).dropna()
    # keep only complete 20m bars (4 x 5m source rows each)
    counts = df5.resample(rule, label="left", closed="left").size()
    return out[counts == RESAMPLE_MIN // 5]


async def fetch(ex, symbol):
    since = ex.parse8601(START.isoformat())
    end_ms = int(END.timestamp() * 1000)
    rows = []
    while True:
        batch = await ex.fetch_ohlcv(symbol, SRC_TF, since=since, limit=1000)
        if not batch:
            break
        rows.extend(batch)
        since = batch[-1][0] + 1
        if len(batch) < 1000 or batch[-1][0] >= end_ms:
            break
        await asyncio.sleep(ex.rateLimit / 1000)
    df = pd.DataFrame(rows, columns=["timestamp", "open", "high", "low", "close", "volume"])
    df["timestamp"] = pd.to_datetime(df["timestamp"], unit="ms")
    df = df[df["timestamp"] < END].set_index("timestamp").sort_index()
    df = df[~df.index.duplicated(keep="first")]
    return resample_20m(df)



async def main():
    ex = ccxt.binance({"enableRateLimit": True})
    os.makedirs(OUT, exist_ok=True)
    try:
        for sym in SYMBOLS:
            df = await fetch(ex, sym)
            safe = sym.replace("/", "_")
            path = os.path.join(OUT, f"{safe}_20m_y2.parquet")
            df.to_parquet(path)
            print(f"{sym}: {len(df)} bars {df.index.min()} -> {df.index.max()}  ({path})")
    finally:
        await ex.close()


if __name__ == "__main__":
    asyncio.run(main())
