"""
S16b — Calibration check for the real-TF re-validation simulator.

Runs the SAME simulator on the FAKE multi-TF model (lookback 20/80/200 on a
single 1h series — the model every prior validation artifact used) and compares
against the REAL multi-TF model. If the simulator reproduces the known-positive
fake-TF result while showing real-TF as negative, the parity gap is confirmed
and the simulator is trustworthy.
"""
import os, sys, json
import numpy as np
import pandas as pd

sys.path.insert(0, os.path.dirname(__file__))
import s16_real_tf_revalidation as s16

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
DATA = os.path.join(ROOT, "data", "ohlcv")


def timeframe_arrays(close, lookback=20):
    sma = close.rolling(lookback).mean()
    trend = np.where(close > sma, 1, np.where(close < sma, -1, 0))
    returns = close.pct_change()
    momentum = returns.rolling(lookback).mean()
    delta = close.diff()
    gain = delta.where(delta > 0, 0.0).rolling(14).mean()
    loss = (-delta.where(delta < 0, 0.0)).rolling(14).mean()
    rs = gain / loss.replace(0, np.nan)
    rsi = (100 - 100 / (1 + rs)).fillna(50.0)
    return trend, momentum, rsi


def build_fake_features():
    """The validated 'fake' model: lookback 20/80/200 on ONE 1h series."""
    df1 = s16.fetch_fresh("1h", "1h", days=380)
    idx = df1.index
    close = df1["close"]
    volume = df1["volume"]

    t1, _, rsi1 = timeframe_arrays(close, 20)
    t4, m4, _ = timeframe_arrays(close, 80)
    td, _, _ = timeframe_arrays(close, 200)

    bull = (t1 == 1).astype(int) + (t4 == 1).astype(int) + (td == 1).astype(int)
    bear = (t1 == -1).astype(int) + (t4 == -1).astype(int) + (td == -1).astype(int)
    vol_ratio = (volume / volume.rolling(20).mean().replace(0, np.nan)).fillna(1.0)
    sma20 = close.rolling(20).mean()
    std20 = close.rolling(20).std()
    bb = ((close - (sma20 - 2 * std20)) / (4 * std20)).fillna(0.5)
    atr = close.pct_change().rolling(20).std() * close
    rsi = pd.Series(rsi1, index=idx)
    mom = pd.Series(m4, index=idx)
    bull = pd.Series(bull, index=idx)
    bear = pd.Series(bear, index=idx)
    daily_bull = pd.Series(td == 1, index=idx)
    daily_bear = pd.Series(td == -1, index=idx)

    scores = {}
    for ma in (2, 3):
        s = pd.Series(0.0, index=idx)
        bl = bull >= ma
        be = bear >= ma
        s = s.where(~bl, s + (bull / 3.0) * 0.4)
        s = s.where(~(bl & (vol_ratio > 1.3)), s + 0.1)
        s = s.where(~be, s - (bear / 3.0) * 0.4)
        s = s.where(~(be & (vol_ratio > 1.3)), s - 0.1)
        mr = pd.Series(0.0, index=idx)
        mr = mr.where(~(rsi < 30), 0.3)
        mr = mr.where(~((rsi >= 30) & (rsi < 35) & daily_bull), 0.2)
        mr = mr.where(~(rsi > 70), -0.3)
        mr = mr.where(~((rsi >= 65) & (rsi <= 70) & daily_bear), -0.2)
        s = s + mr * 0.3
        s = s + np.where(mom > 0.003, 0.15, np.where(mom < -0.003, -0.15, 0.0))
        s = s + np.where(bb < 0.15, 0.15, np.where(bb > 0.85, -0.15, 0.0))
        scores[ma] = s

    return dict(close=close, atr=atr, rsi=rsi, score2=scores[2], score3=scores[3],
                bull=bull, bear=bear)


def main():
    print("=== CALIBRATION: same simulator, fake-TF vs real-TF ===\n")
    fake = build_fake_features()
    real = s16.build_features()
    n = len(fake["close"])
    print(f"window: {fake['close'].index[0]} -> {fake['close'].index[-1]} ({n} bars)\n")

    configs = [
        ("baseline (0.30/align2/trail1.0)", dict(s16.BASELINE)),
        ("override (0.24/align2/trail1.0)", dict(s16.BASELINE, signal_threshold=0.24)),
        ("night_top (0.30/align3/trail1.0)", dict(s16.BASELINE, min_alignment=3)),
    ]
    print(f"{'config':34s} {'model':6s} {'medSharpe':>9s} {'netPnL%':>9s} {'cons':>5s} {'trades':>6s}")
    print("-" * 80)
    for label, p in configs:
        for name, feat in [("FAKE", fake), ("REAL", real)]:
            r = s16.evaluate(feat, p)
            print(f"{label:34s} {name:6s} {r['median_sharpe']:9.2f} {r['total_net_pct']:9.1f} "
                  f"{r['consistency']*100:4.0f}% {r['trades']:6d}")
        print()


if __name__ == "__main__":
    sys.exit(main())
