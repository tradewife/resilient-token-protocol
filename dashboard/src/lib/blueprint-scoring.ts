/** Resilience Blueprint scoring engine.
 *
 * Computes 4 scores (0–10) from the 12 questionnaire answers and generates
 * a personalised profile with archetype, custody stance, and commitment hint.
 */

export interface BlueprintAnswers {
  q1_venues: string[]; // multi: cex, trad_broker, onchain, mix
  q2_account_size: string; // radio: <10k, 10k-50k, 50k-250k, >250k
  q3_activity: string; // radio: daily, several_week, weekly, passive
  q4_drawdown: string; // radio: <10%, 10-20%, 20-35%, >35%
  q5_pain_points: string[]; // multi: liquidations, choppy, funding_bleed, venue_outages, scam_coins, other
  q6_risk_orientation: string; // radio: avoid_downswings, maximize_growth, balanced
  q7_custody_comfort: string; // radio: not_at_all, somewhat, comfortable, very_comfortable
  q8_custody_setup: string; // radio: own_wallet, program_vaults, mix, not_sure
  q9_cadence: string; // radio: weekly, major_changes, monthly, live_dashboard
  q10_goal: string; // radio: compounding, income, directional, hedge
  q11_do_not_do: string[]; // multi: leverage_above, illiquid, overnight, outside_solana, other
  q12_commitment: string; // radio: ready, probably, not_yet
}

export interface BlueprintProfile {
  onChainReadiness: number;
  riskTolerance: number;
  complexityAppetite: number;
  commitmentReadiness: number;
  onChainLabel: string;
  onChainExplanation: string;
  riskSummary: string;
  archetype: string;
  archetypeDescription: string;
  custodyStance: string;
  commitmentHint: string;
}

/* ── Question→Score mapping utilities ── */

function pick(answers: BlueprintAnswers, q: string): string {
  const v = (answers as unknown as Record<string, unknown>)[q];
  if (Array.isArray(v)) return v.join(",");
  return String(v ?? "");
}

function has(answers: BlueprintAnswers, q: string, val: string): boolean {
  const arr = (answers as unknown as Record<string, unknown>)[q] as string[] | undefined;
  return Array.isArray(arr) && arr.includes(val);
}

function count(answers: BlueprintAnswers, q: string): number {
  const arr = (answers as unknown as Record<string, unknown>)[q] as string[] | undefined;
  return Array.isArray(arr) ? arr.length : 0;
}

/* ── Score computation ── */

function computeOnChainReadiness(answers: BlueprintAnswers): number {
  let score = 0;
  let div = 0;

  // Q1: venue decentralization
  const v = pick(answers, "q1_venues");
  if (v) {
    // Multi-select stored as comma string from the form; handle both
    const venues = v.includes(",") ? v.split(",").map((s) => s.trim()) : [v];
    let max = 0;
    for (const venue of venues) {
      if (venue === "onchain") max = Math.max(max, 9);
      else if (venue === "mix") max = Math.max(max, 5);
      else if (venue === "cex") max = Math.max(max, 2);
      else if (venue === "trad_broker") max = Math.max(max, 2);
    }
    score += max;
    div++;
  }

  // Q7: custody comfort
  switch (pick(answers, "q7_custody_comfort")) {
    case "very_comfortable":
      score += 10;
      break;
    case "comfortable":
      score += 7;
      break;
    case "somewhat":
      score += 4;
      break;
    default:
      score += 1;
      break;
  }
  div++;

  // Q8: custody setup
  switch (pick(answers, "q8_custody_setup")) {
    case "program_vaults":
      score += 10;
      break;
    case "own_wallet":
      score += 8;
      break;
    case "mix":
      score += 6;
      break;
    default:
      score += 3;
      break;
  }
  div++;

  return Math.round(score / div);
}

function computeRiskTolerance(answers: BlueprintAnswers): number {
  let score = 0;
  let div = 0;

  // Q4: drawdown
  switch (pick(answers, "q4_drawdown")) {
    case ">35%":
      score += 10;
      break;
    case "20-35%":
      score += 8;
      break;
    case "10-20%":
      score += 5;
      break;
    default:
      score += 3;
      break;
  }
  div++;

  // Q6: risk orientation
  switch (pick(answers, "q6_risk_orientation")) {
    case "maximize_growth":
      score += 9;
      break;
    case "balanced":
      score += 5;
      break;
    default:
      score += 2;
      break;
  }
  div++;

  // Q11: dampen by do-not-do count
  const dndCount = count(answers, "q11_do_not_do");
  const dampener = Math.max(0.5, 1 - dndCount * 0.1);
  return Math.round(((score / div) * dampener));
}

function computeComplexityAppetite(answers: BlueprintAnswers): number {
  let score = 0;
  let div = 0;

  // Q3: activity
  switch (pick(answers, "q3_activity")) {
    case "daily":
      score += 9;
      break;
    case "several_week":
      score += 7;
      break;
    case "weekly":
      score += 4;
      break;
    default:
      score += 2;
      break;
  }
  div++;

  // Q9: cadence
  switch (pick(answers, "q9_cadence")) {
    case "weekly":
      score += 8;
      break;
    case "live_dashboard":
      score += 5;
      break;
    case "monthly":
      score += 3;
      break;
    default:
      score += 3;
      break;
  }
  div++;

  // Q10: goal aggressiveness
  switch (pick(answers, "q10_goal")) {
    case "directional":
      score += 8;
      break;
    case "income":
      score += 5;
      break;
    case "compounding":
      score += 4;
      break;
    default:
      score += 3;
      break;
  }
  div++;

  return Math.round(score / div);
}

function computeCommitmentReadiness(answers: BlueprintAnswers): number {
  switch (pick(answers, "q12_commitment")) {
    case "ready":
      return 10;
    case "probably":
      return 5;
    default:
      return 2;
  }
}

/* ── Profile label generation ── */

function generateOnChainLabel(score: number): { label: string; explanation: string } {
  if (score >= 8) {
    return {
      label: "On-chain native — comfortable with self-custody and advanced venues.",
      explanation:
        "You trade on-chain, understand custody, and are comfortable with program-derived vaults. Step zero for you is selecting the optimal venue and custody configuration — not learning the basics.",
    };
  }
  if (score >= 5) {
    return {
      label: "Mixed venues, on-chain intermediate.",
      explanation:
        "You have some on-chain experience but may still default to centralised venues for most activity. Step zero is a guided walk-through of self-custody tooling and on-chain perp venue mechanics so you can operate with confidence.",
    };
  }
  return {
    label: "CEX-native, on-chain novice.",
    explanation:
      "You primarily trade on centralised exchanges. Step zero is a guided self-custody setup (hardware wallet, seed safety, program vault concepts) plus venue selection before any strategy touches capital.",
  };
}

function generateRiskSummary(answers: BlueprintAnswers, score: number): string {
  const dd = pick(answers, "q4_drawdown");
  const drawdownLabel = dd.replace("-", "–").replace(">", ">").replace("<", "<");

  const painPoints = Array.isArray(answers.q5_pain_points)
    ? answers.q5_pain_points
        .map((p) => {
          const map: Record<string, string> = {
            liquidations: "liquidations",
            choppy: "choppy markets",
            funding_bleed: "funding bleed",
            venue_outages: "venue outages",
            scam_coins: "illiquid/scam coins",
            other: "other past pain",
          };
          return map[p] || p;
        })
        .join(" and ")
    : "past market events";

  let stance: string;
  if (score >= 7) stance = "aggressive";
  else if (score >= 4) stance = "balanced";
  else stance = "conservative";

  return `You flagged ~${drawdownLabel} as unacceptable and highlighted ${painPoints} as prior pain points. I'd treat your engine as ${stance} with hard caps and routes around those failure modes.`;
}

function generateArchetype(
  answers: BlueprintAnswers,
  onChain: number,
  risk: number,
  complexity: number
): { archetype: string; description: string } {
  const goal = pick(answers, "q10_goal");

  if (goal === "hedge") {
    return {
      archetype: "Treasury Hedging Engine",
      description:
        "An engine designed to offset directional risk on existing holdings rather than seek standalone alpha. It uses inverse or low-correlation positions on Solana perps, sized to your hedge ratio, with hard drawdown stops and a do-not-trade list baked into the config. Positions are opened only when the underlying exposure crosses a pre-defined threshold.",
    };
  }

  if (onChain >= 6 && risk >= 6 && complexity >= 6) {
    return {
      archetype: "Directional Engine with Pre-defined Max Drawdown and Leverage Caps",
      description:
        "A higher-octane engine that takes directional bets on Solana perps with strict position limits, maximum leverage caps, and a hard drawdown floor. The strategy uses multi-timeframe confluence for entries and a trailing stop + take-profit exit stack. Designed for traders who want conviction-sized exposure without unbounded risk.",
    };
  }

  if (onChain >= 5 && complexity >= 5) {
    return {
      archetype: "Regime-Rotation Perps Engine Targeting Long-Run Compounding",
      description:
        "A multi-strategy engine that rotates between trend-following and mean-reversion regimes based on on-chain volatility and funding-rate signals. Positions are sized conservatively with a bias toward compounding over years. The engine sits flat during unfavourable regimes rather than forcing activity.",
    };
  }

  return {
    archetype: "Conservative Yield Engine on Solana Perps with Strict Position Limits",
    description:
      "A capital-preservation-first engine that uses delta-neutral or low-delta positioning on Solana perps, capturing funding-rate spreads and mild directional edge with tight stops. Position sizes are capped at a fraction of portfolio, and the engine goes flat whenever volatility exceeds a pre-configured threshold. Designed for steady, low-drawdown accumulation.",
  };
}

function generateCustodyStance(answers: BlueprintAnswers): string {
  const setup = pick(answers, "q8_custody_setup");
  if (setup === "program_vaults" || setup === "own_wallet") {
    return "Funds live either in your own wallet or in on-chain program vaults with no operator-held keys; you stay in control.";
  }
  if (setup === "mix") {
    return "A mix of self-custodied wallets and on-chain program vaults — you retain control with the flexibility to tap programmatic safeguards.";
  }
  return "You'd benefit from guidance on custody; we can walk through wallet, vault, and hybrid models before committing to a setup.";
}

function generateCommitmentHint(answers: BlueprintAnswers): string {
  switch (pick(answers, "q12_commitment")) {
    case "ready":
      return "You indicated you're ready to commit now — the next step is securing a build slot.";
    case "probably":
      return "You're close; I'll offer an option to ask a couple of questions before you commit.";
    default:
      return "Take your time. When you're ready, the build slot is here.";
  }
}

/* ── Public API ── */

export function computeBlueprintProfile(answers: BlueprintAnswers): BlueprintProfile {
  const onChainReadiness = computeOnChainReadiness(answers);
  const riskTolerance = computeRiskTolerance(answers);
  const complexityAppetite = computeComplexityAppetite(answers);
  const commitmentReadiness = computeCommitmentReadiness(answers);

  const onChain = generateOnChainLabel(onChainReadiness);
  const riskSummary = generateRiskSummary(answers, riskTolerance);
  const archetype = generateArchetype(answers, onChainReadiness, riskTolerance, complexityAppetite);
  const custodyStance = generateCustodyStance(answers);
  const commitmentHint = generateCommitmentHint(answers);

  return {
    onChainReadiness,
    riskTolerance,
    complexityAppetite,
    commitmentReadiness,
    onChainLabel: onChain.label,
    onChainExplanation: onChain.explanation,
    riskSummary,
    archetype: archetype.archetype,
    archetypeDescription: archetype.description,
    custodyStance,
    commitmentHint,
  };
}
