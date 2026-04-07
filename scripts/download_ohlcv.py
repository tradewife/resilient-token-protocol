"""Download historical OHLCV data from Binance via ccxt"""
import asyncio
import ccxt.async_support as ccxt
import pandas as pd
import json
import os
from datetime import datetime

SYMBOLS = ["BTC/USDT", "ETH/USDT", "SOL/USDT", "BNB/USDT", "XRP/USDT"]
TIMEFRAMES = ["1h", "4h", "1d"]
DAYS_BACK = 365
OUTPUT_DIR = os.path.join(os.path.dirname(__file__), "..", "data", "ohlcv")

async def download_symbol(exchange, symbol, timeframe, days_back):
    """Download OHLCV for one symbol/timeframe"""
    since = exchange.parse8601(
        (datetime.utcnow() - pd.Timedelta(days=days_back)).isoformat()
    )
    all_ohlcv = []
    while True:
        batch = await exchange.fetch_ohlcv(symbol, timeframe, since=since, limit=1000)
        if not batch:
            break
        all_ohlcv.extend(batch)
        since = batch[-1][0] + 1
        if len(batch) < 1000:
            break
        await asyncio.sleep(exchange.rateLimit / 1000)

    df = pd.DataFrame(all_ohlcv, columns=["timestamp", "open", "high", "low", "close", "volume"])
    df["timestamp"] = pd.to_datetime(df["timestamp"], unit="ms")
    df.set_index("timestamp", inplace=True)
    return df

async def main():
    exchange = ccxt.binance({"enableRateLimit": True})
    os.makedirs(OUTPUT_DIR, exist_ok=True)

    tasks = []
    for symbol in SYMBOLS:
        for tf in TIMEFRAMES:
            tasks.append((symbol, tf, download_symbol(exchange, symbol, tf, DAYS_BACK)))

    results = {}
    for symbol, tf, coro in tasks:
        try:
            df = await coro
            safe_name = symbol.replace("/", "_")
            path = os.path.join(OUTPUT_DIR, f"{safe_name}_{tf}.parquet")
            df.to_parquet(path)
            results[f"{symbol}_{tf}"] = len(df)
            print(f"  {symbol:12s} {tf:4s} → {len(df):6d} candles  {df.index[0].date()} to {df.index[-1].date()}")
        except Exception as e:
            print(f"  {symbol:12s} {tf:4s} → FAILED: {e}")

    await exchange.close()
    print(f"\nDownloaded {len(results)} datasets")
    # Save metadata
    meta = {
        "downloaded_at": datetime.utcnow().isoformat(),
        "exchange": "binance",
        "days_back": DAYS_BACK,
        "datasets": {k: v for k, v in results.items()},
    }
    with open(os.path.join(OUTPUT_DIR, "metadata.json"), "w") as f:
        json.dump(meta, f, indent=2)

if __name__ == "__main__":
    asyncio.run(main())
