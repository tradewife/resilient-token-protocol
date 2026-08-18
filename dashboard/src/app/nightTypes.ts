/** Shape of `/data/night.json` — written by the Night Shift pipeline. */

export interface NightCandidate {
  symbol: string;
  survivor_score: number;
  oos_sharpe: number;
  oos_consistency: number;
  oos_max_dd: number;
  overfitting_score: number;
  fragility: number;
}

export interface NightData {
  num_folds: number;
  runtime_seconds: number;
  top_candidates: NightCandidate[];
}
