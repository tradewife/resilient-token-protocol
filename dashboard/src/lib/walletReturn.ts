/**
 * Specimen wallet return — cash-in vs live NAV, including SOL hold and
 * any open GMTrade position.
 *
 * The closed-tape "equity compounded" figure in tradePnl.ts assumes every
 * trade risks 20% × 9× of a growing wallet. Live collateral is not that,
 * so that curve is not account return. This module is.
 *
 * Capital events are the on-chain reconstruction (Apr–Aug 2026):
 * external deposits into Driyi8 / HDQ79, one user withdrawal, excluding
 * Flash/GMTrade wrap, Driyi8→HDQ79 rotations, and the 1 SOL incinerator
 * burn (already reflected in remaining NAV).
 */

export const SPECIMEN_LEVERAGE = 9;

export const LEGACY_TRADER_WALLET =
  "Driyi8Sw2622yCefU34zrjBsQynrDoGD31tBecXrEF6R";

export type CapitalEvent = {
  t: string;
  kind: "deposit" | "withdrawal";
  sol: number;
  usdPerSol: number;
  sig: string;
  note: string;
};

export const SPECIMEN_CAPITAL_EVENTS: readonly CapitalEvent[] = [
  {
    t: "2026-04-27T09:08:11Z",
    kind: "deposit",
    sol: 0.055987,
    usdPerSol: 85.18,
    sig: "23ZPJdiVJtRgcMZgkBN5joRt49zYpp4HmxAegju4GMtmsiKdCQqzCSZEuBGvL9B2eCbULu5YVxuYdtMj8WPUyYN1",
    note: "HWjmoU → Driyi8",
  },
  {
    t: "2026-05-04T21:36:53Z",
    kind: "deposit",
    sol: 2.005555,
    usdPerSol: 84.22,
    sig: "3d7ooANtSTVhQHx39saUppXcfuEgDcbfrjWpaw1V8u6KB8EMYjfSLSwEvcCx9hx4CWZS59KfvkushF46psaFXjgN",
    note: "HWjmoU → Driyi8",
  },
  {
    t: "2026-08-05T16:17:39Z",
    kind: "deposit",
    sol: 2.050464,
    usdPerSol: 74.55,
    sig: "PFveVTLJNzMMjbKXA8CKxu5SYCosYmoeDuiKQUhJoLUneyM9bkmjjBFXSFMM63VNBGVN2RhXEecXq9tpyP4uExg",
    note: "deBridge FinalizeTransferSol → Driyi8",
  },
  {
    t: "2026-07-23T03:11:24Z",
    kind: "withdrawal",
    sol: 1.0,
    usdPerSol: 77.53,
    sig: "4vdEmf2rQZ5yBDkooM",
    note: "HDQ79 → HRSohn83",
  },
];

export const SPECIMEN_DEPOSITS_USD = SPECIMEN_CAPITAL_EVENTS.filter(
  (e) => e.kind === "deposit",
).reduce((s, e) => s + e.sol * e.usdPerSol, 0);

export const SPECIMEN_WITHDRAWALS_USD = SPECIMEN_CAPITAL_EVENTS.filter(
  (e) => e.kind === "withdrawal",
).reduce((s, e) => s + e.sol * e.usdPerSol, 0);

export type OpenPositionLike = {
  entry_price: number;
  size_usd: number;
  side?: string;
} | null | undefined;

export type WalletReturnInput = {
  nativeSol: number | null;
  legacySol: number;
  spotUsd: number | null;
  openPosition: OpenPositionLike;
};

export type WalletReturn = {
  returnPct: number;
  navUsd: number;
  navSol: number;
  nativeUsd: number;
  positionEquityUsd: number;
  contributedUsd: number;
  withdrawnUsd: number;
};

export function openPositionEquityUsd(
  pos: OpenPositionLike,
  spotUsd: number,
  leverage = SPECIMEN_LEVERAGE,
): number {
  if (!pos || !(pos.size_usd > 0) || !(pos.entry_price > 0) || !(spotUsd > 0)) {
    return 0;
  }
  const collateralUsd = pos.size_usd / leverage;
  const side = (pos.side ?? "Long").toLowerCase();
  const dir = side.startsWith("s") ? -1 : 1;
  const unrealizedUsd =
    pos.size_usd * dir * ((spotUsd - pos.entry_price) / pos.entry_price);
  return collateralUsd + unrealizedUsd;
}

/**
 * USD return on cash contributed:
 *   (live NAV + USD withdrawn − USD deposited) / USD deposited
 *
 * Live NAV = native SOL (both wallets) × spot + open-position equity
 * (collateral that left the wallet + mark-to-market).
 */
export function summarizeWalletReturn(
  input: WalletReturnInput,
): WalletReturn | null {
  const { nativeSol, legacySol, openPosition } = input;
  const spot = input.spotUsd;
  if (nativeSol == null || spot == null || !(spot > 0) || nativeSol < 0) {
    return null;
  }

  const nativeUsd = (nativeSol + legacySol) * spot;
  const positionEquityUsd = openPositionEquityUsd(openPosition, spot);
  const navUsd = nativeUsd + positionEquityUsd;
  if (!(SPECIMEN_DEPOSITS_USD > 0)) return null;

  const returnPct =
    ((navUsd + SPECIMEN_WITHDRAWALS_USD - SPECIMEN_DEPOSITS_USD) /
      SPECIMEN_DEPOSITS_USD) *
    100;

  return {
    returnPct,
    navUsd,
    navSol: navUsd / spot,
    nativeUsd,
    positionEquityUsd,
    contributedUsd: SPECIMEN_DEPOSITS_USD,
    withdrawnUsd: SPECIMEN_WITHDRAWALS_USD,
  };
}
