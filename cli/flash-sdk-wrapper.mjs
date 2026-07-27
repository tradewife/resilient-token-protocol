#!/usr/bin/env node
/**
 * Flash Trade SDK v2 Wrapper
 *
 * JSON-RPC over stdin/stdout for communication with the Rust trader.
 * Reads keypair from RTP_TRADER_KEYPAIR_JSON (never argv).
 * Exposes: setup, open_position, close_position, get_price
 *
 * SDK pin: @flash_trade/flash-sdk-v2@1.0.36 — APIs called here are validated
 * against the bundled dist/index.d.ts. See node_modules/@flash_trade/flash-sdk-v2/dist/FlashPerpetualsClient.d.ts
 * for the exact signatures.
 */

import {
  FlashPerpetualsClient,
  PROGRAM_ID,
  PoolConfig,
  Side,
  isVariant,
} from "@flash_trade/flash-sdk-v2";
import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import { AnchorProvider, Wallet } from "@coral-xyz/anchor";
import BN from "bn.js";

const SOL_MINT = new PublicKey("So11111111111111111111111111111111111111112");
const CLUSTER = "mainnet-beta";
const ER_RPC = process.env.RTP_TRADER_ER_RPC || "https://flash.magicblock.xyz";
const RPC_URL = process.env.RTP_SOLANA_RPC_URL || "https://api.mainnet-beta.solana.com";

const BTC_DECIMALS_BPS = 10000; // BPS_DECIMALS = 4 — leverage stored as leverage * BTC_DECIMALS_BPS
const DEPOSIT_SKIP_FUNDED_LAMPORTS = 50_000_000; // 0.05 SOL residual: don't top up an already-funded ledger below this
const DEFAULT_OPEN_BACKOFF_ATTEMPTS = 8;
const DEFAULT_MIN_OPEN_COLLATERAL_LAMPORTS = 5_000_000; // 0.005 SOL

let client = null;
let keypair = null;
let provider = null;
let poolConfig = null;

function sideLong() { return Side.Long; }
function sideShort() { return Side.Short; }

async function initClient() {
  if (client) return client;

  const secretJson = process.env.RTP_TRADER_KEYPAIR_JSON;
  if (!secretJson) throw new Error("RTP_TRADER_KEYPAIR_JSON not set");

  const secret = JSON.parse(secretJson);
  keypair = Keypair.fromSecretKey(new Uint8Array(secret));

  const connection = new Connection(RPC_URL, "confirmed");
  const wallet = new Wallet(keypair);
  provider = new AnchorProvider(connection, wallet, { commitment: "confirmed" });

  poolConfig = PoolConfig.fromIdsByName("Crypto.1", CLUSTER);

  client = new FlashPerpetualsClient(
    provider,
    undefined,
    PROGRAM_ID[CLUSTER],
    { prioritizationFee: 5000, txConfirmationCommitment: "confirmed" },
    ER_RPC,
  );

  console.error(
    "[wrapper] Flash SDK v2 initialized, program:",
    client.programId?.toBase58?.() ?? "unknown",
    "pool:",
    poolConfig.poolName ?? "Crypto.1",
  );

  return client;
}

function formatError(e) {
  if (e == null) return "unknown error";
  if (typeof e === "string") return e;
  if (typeof e?.message === "string" && e.message.length) return e.message;
  if (typeof e?.message === "object") {
    try {
      return JSON.stringify(e.message);
    } catch {
      /* fall through */
    }
  }
  try {
    return JSON.stringify(e);
  } catch {
    return String(e);
  }
}

// JSON-RPC request handler
async function handleRequest(req) {
  const { method, params, id } = req;
  try {
    let result;
    switch (method) {
      case "setup":
        result = await doSetup();
        break;
      case "open_position":
        result = await doOpenPosition(params);
        break;
      case "close_position":
        result = await doClosePosition(params);
        break;
      case "get_price":
        result = await doGetPrice(params);
        break;
      case "readiness":
        result = await readTradeReadiness(await initClient());
        break;
      default:
        throw new Error(`Unknown method: ${method}`);
    }
    return { jsonrpc: "2.0", id, result };
  } catch (e) {
    const message = formatError(e);
    console.error("[wrapper] request error:", message);
    return { jsonrpc: "2.0", id, error: { code: -32000, message } };
  }
}

// Resolve the first configured market for (targetSymbol, side). This mirrors the
// docs snippet and is used as a base before applying SDK open-only overrides.
function getMarket(targetSymbol, side) {
  const targetToken = poolConfig.getTokenFromSymbol(targetSymbol);
  const targetCustody = poolConfig.custodies.find((c) =>
    c.mintKey.equals(targetToken.mintKey),
  );
  if (!targetCustody) throw new Error(`no custody for ${targetSymbol}`);
  const market = poolConfig.markets.find(
    (m) =>
      m.targetCustody.equals(targetCustody.custodyAccount) &&
      isVariant(m.side, "long") === isVariant(side, "long"),
  );
  if (!market) {
    throw new Error(
      `no ${isVariant(side, "long") ? "long" : "short"} market for ${targetSymbol}`,
    );
  }
  const collateral = poolConfig.custodies.find((c) =>
    c.custodyAccount.equals(market.collateralCustody),
  );
  if (!collateral) throw new Error("collateral custody not found for market");
  return { market: market.marketAccount, side, collateralSymbol: collateral.symbol };
}

function getCustodyBySymbol(symbol) {
  const token = poolConfig.getTokenFromSymbol(symbol);
  const custody = poolConfig.custodies.find((c) => c.mintKey.equals(token.mintKey));
  if (!custody) throw new Error(`no custody for ${symbol}`);
  return custody;
}

function getMarketForLock(targetSymbol, lockSymbol, side) {
  const targetCustody = getCustodyBySymbol(targetSymbol);
  const lockCustody = getCustodyBySymbol(lockSymbol);
  const market =
    typeof client?.findMarketConfig === "function"
      ? client.findMarketConfig(poolConfig, targetSymbol, lockSymbol, side)
      : poolConfig.getMarketConfig(targetCustody.custodyAccount, lockCustody.custodyAccount, side);
  if (!market) {
    throw new Error(
      `no ${isVariant(side, "long") ? "long" : "short"} market for ${targetSymbol}/${lockSymbol}`,
    );
  }
  return { market: market.marketAccount, side, collateralSymbol: lockSymbol };
}

function resolveOpenMarket(targetSymbol, side) {
  const base = getMarket(targetSymbol, side);
  const lockSymbol =
    typeof client?.resolveCollateralSymbol === "function"
      ? client.resolveCollateralSymbol(targetSymbol, base.collateralSymbol, side)
      : base.collateralSymbol;
  const resolved = getMarketForLock(targetSymbol, lockSymbol, side);
  return {
    market: resolved.market,
    lockSymbol,
    fundingSymbol: base.collateralSymbol,
  };
}

/** Coerce Anchor account fields / numbers into BN for SDK math helpers. */
function toBn(v) {
  if (BN.isBN?.(v) || (v && typeof v.toNumber === "function" && typeof v.toString === "function" && v.constructor?.name === "BN")) {
    return v;
  }
  if (typeof v === "number") return new BN(Math.trunc(v));
  if (typeof v === "bigint") return new BN(v.toString());
  if (v != null && typeof v.toString === "function") return new BN(v.toString());
  throw new Error(`cannot convert to BN: ${typeof v} ${v}`);
}

// Fetch the oracle price for `targetSymbol` from the matching custody and return it
// as a {price: BN, exponent: BN} pair — getPriceAfterSlippage calls
// targetPrice.exponent.toNumber() (SDK v1.0.36).
async function readOraclePrice(targetSymbol) {
  const c = await initClient();
  const targetToken = poolConfig.getTokenFromSymbol(targetSymbol);
  const custody = poolConfig.custodies.find((c) =>
    c.mintKey.equals(targetToken.mintKey),
  );
  if (!custody) throw new Error(`no custody for ${targetSymbol}`);
  const program = c.erProgram ?? c.program;
  const oracle = await program.account.customOracle.fetch(custody.intOracleAccount);
  // Anchor may decode price/expo as BN or as number depending on IDL/codegen.
  const rawPrice = oracle.price ?? oracle.priceUi;
  const rawExpo = oracle.expo ?? oracle.exponent;
  return { price: toBn(rawPrice), exponent: toBn(rawExpo) };
}

// Build a slippage-bounded ContractOraclePrice.
async function entryPrice(targetSymbol, side, isEntry, slippageBps = 100) {
  const c = await initClient();
  const tp = await readOraclePrice(targetSymbol);
  return c.getPriceAfterSlippage(isEntry, new BN(slippageBps), tp, side);
}

function depositEntryBalance(entry) {
  const bal = entry?.balance ?? entry?.amount ?? entry?.lamports ?? 0;
  try {
    return toBn(bal);
  } catch {
    return new BN(0);
  }
}

function solDepositBalance(ledger) {
  const entries = ledger?.deposits ?? ledger?.entries ?? [];
  if (!Array.isArray(entries)) return new BN(0);
  const solEntry = entries.find((d) => {
    const mint = d?.mintKey ?? d?.tokenMint ?? d?.mint;
    try {
      return mint && new PublicKey(mint).equals(SOL_MINT);
    } catch {
      return false;
    }
  });
  if (!solEntry) return new BN(0);
  return depositEntryBalance(solEntry);
}

/** True when the deposit ledger has any non-zero mint balance (SOL, USDC, …). */
function anyDepositFunded(ledger) {
  const entries = ledger?.deposits ?? ledger?.entries ?? [];
  if (!Array.isArray(entries) || entries.length === 0) return false;
  return entries.some((d) => depositEntryBalance(d).gt(new BN(0)));
}

/**
 * Readiness for Flash v2 trading.
 *
 * Required: non-empty deposit ledger (opens fail with 6024 when unfunded).
 * Advisory: basket.delegate — successful mainnet opens use session keys with
 * delegate still unset (1111…); MagicBlock ownership of the basket is enough.
 */
async function readTradeReadiness(c) {
  const out = {
    depositLamports: "0",
    depositOk: false,
    anyDepositOk: false,
    depositMints: [],
    basketDelegate: null,
    flashDelegated: false,
    basketOk: false,
    ready: false,
    issues: [],
  };
  try {
    const ledger = await c.accounts.fetchUserDepositLedger(keypair.publicKey);
    const bal = solDepositBalance(ledger);
    out.depositLamports = bal.toString();
    out.anyDepositOk = anyDepositFunded(ledger);
    // SOL path (our trader collateral) needs SOL on the ledger. Accept any
    // funded mint for setup diagnostics, but ready requires SOL for SOL opens.
    out.depositOk = bal.gt(new BN(0));
    const entries = ledger?.deposits ?? ledger?.entries ?? [];
    if (Array.isArray(entries)) {
      out.depositMints = entries.map((d) => {
        const mint = d?.mintKey ?? d?.tokenMint ?? d?.mint;
        return {
          mint: mint?.toBase58?.() ?? String(mint ?? ""),
          amount: depositEntryBalance(d).toString(),
        };
      });
    }
    if (!out.depositOk) {
      out.issues.push(
        out.anyDepositOk
          ? `deposit ledger has non-SOL balances only; SOL balance is 0 (trader opens need SOL or switch funding mint)`
          : `deposit ledger empty (0 deposits). Flash deposit_direct SDK instruction returns InstructionFallbackNotFound (101) post-upgrade — use POST /transaction-builder/deposit (API one-shot) instead`,
      );
    }
  } catch (e) {
    out.issues.push(`deposit ledger unreadable: ${e?.message ?? e}`);
  }

  try {
    if (!c.erAccounts) {
      out.issues.push("ER accounts client unavailable (set RTP_TRADER_ER_RPC)");
    } else {
      const basket = await c.erAccounts.fetchBasket(keypair.publicKey);
      const del = basket?.delegate?.toBase58?.() ?? String(basket?.delegate ?? "");
      out.basketDelegate = del;
      // Flash-level trading delegate field (legacy). Unset is OK with owner/session signing.
      out.flashDelegated = Boolean(del && del !== PublicKey.default.toBase58());
      out.basketOk = true;
      if (!out.flashDelegated) {
        // Advisory only. The `basket.delegate` field is deprecated per the new
        // SDK (BasketAccount.deprecatedDelegate). Opens use session keys, not
        // basket.delegate. Setting basket.delegate on ER surfaces Custom:27
        // (UnsupportedToken) — Flash pool config on ER does not include the
        // user's basket account, so any delegateBasket-from-ER instruction is
        // rejected. Confirmed empirically: setting basket.delegate is not
        // necessary for opens to land successfully. Do NOT block readiness.
        out.issues.push(
          "basket.delegate unset (deprecated field; opens use session keys instead — see BasketAccount.deprecatedDelegate)",
        );
      }
    }
  } catch (e) {
    out.issues.push(`basket unreadable on ER: ${e?.message ?? e}`);
  }

  // Gate on funded SOL deposit + readable basket. Do not require flashDelegated.
  out.ready = out.depositOk && out.basketOk;
  return out;
}

// Setup/funds → Solana RPC (sendAndConfirmTransaction). Trading → ER.
//
// The SDK's depositDirect() calls the bare on-chain instruction which returns
// InstructionFallbackNotFound (101) after the Squads upgrade at slot 434407053
// (2026-07-22). The REST API's POST /transaction-builder/deposit builds a
// composite 4-instruction tx (system → token → flash → token) that bundles
// any missing setup (basket, deposit ledger, delegation, trade vault) and
// works with the deployed binary. We use the API path for funding.
async function doSetup() {
  const c = await initClient();
  const sigs = [];
  const depositUi = process.env.RTP_TRADER_V2_DEPOSIT_AMOUNT_UI || "1.0";

  // Check if ledger is already funded — skip deposit if so.
  let skipDeposit = false;
  try {
    const ledger = await c.accounts.fetchUserDepositLedger(keypair.publicKey);
    if (solDepositBalance(ledger).gte(new BN(DEPOSIT_SKIP_FUNDED_LAMPORTS))) {
      skipDeposit = true;
    }
  } catch (e) {
    console.error("[wrapper] ledger read failed, will attempt API deposit:", e?.message);
  }

  if (skipDeposit) {
    sigs.push({
      step: "deposit",
      signature: null,
      skipped: `ledger-funded (>=${DEPOSIT_SKIP_FUNDED_LAMPORTS} lamports)`,
    });
  } else {
    // One-shot POST /transaction-builder/deposit — bundles init + deposit.
    // Submits to Solana mainnet RPC (funds ops, not ER).
    const api = process.env.FLASH_API_URL || "https://flashapi.trade";
    const rpc = process.env.RTP_SOLANA_RPC_URL || RPC_URL;
    console.error(`[wrapper] depositing ${depositUi} SOL via API /deposit (one-shot)`);
    const body = {
      owner: keypair.publicKey.toBase58(),
      tokenSymbol: "SOL",
      amount: depositUi,
    };
    const res = await fetch(`${api}/transaction-builder/deposit`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    const j = await res.json();
    if (!j.transactionBase64) {
      throw new Error(
        `API /deposit failed (status ${res.status}): ${JSON.stringify(j).substring(0, 300)}`,
      );
    }
    // Sign and submit to Solana mainnet RPC.
    const { VersionedTransaction, Connection } = await import("@solana/web3.js");
    const tx = VersionedTransaction.deserialize(
      Buffer.from(j.transactionBase64, "base64"),
    );
    tx.sign([keypair]);
    const conn = new Connection(rpc, "confirmed");
    const { blockhash, lastValidBlockHeight } = await conn.getLatestBlockhash("confirmed");
    const sig = await conn.sendRawTransaction(tx.serialize(), {
      skipPreflight: false,
      maxRetries: 3,
    });
    await conn.confirmTransaction(
      { signature: sig, blockhash, lastValidBlockHeight },
      "confirmed",
    );
    console.error(`[wrapper] deposit confirmed: ${sig}`);
    sigs.push({ step: "deposit", signature: sig });
  }

  // SDK delegateBasket no-ops when the basket account is MagicBlock-owned on L1,
  // even if Flash's basket.delegate field is still unset. Force Flash-level
  // activation on ER when readiness says we're not flash-delegated.
  const before = await readTradeReadiness(c);
  if (before.flashDelegated) {
    sigs.push({ step: "delegate-basket", signature: null, skipped: "already-flash-delegated" });
  } else {
    // Prefer SDK path first (base). If it no-ops, force ER send of the ix.
    let r;
    try {
      r = await c.delegateBasket(keypair.publicKey);
    } catch (e) {
      const msg = e?.message ?? String(e);
      if (isProgramMismatchError(msg)) {
        throw new Error(
          `delegate-basket failed: Flash program InstructionFallbackNotFound (Custom 101). ` +
            `Deployed program does not recognize delegate_basket. Original: ${msg}`,
        );
      }
      throw e;
    }
    if (r?.instructions?.length) {
      try {
        const sig = await c.sendAndConfirmTransaction(r.instructions, {
          additionalSigners: r.additionalSigners ?? [],
        });
        sigs.push({ step: "delegate-basket", signature: sig });
      } catch (e) {
        // Basket is often already MagicBlock-delegated on L1; Flash-level
        // delegate must land on ER.
        console.error(
          "[wrapper] base delegate-basket failed, retrying on ER:",
          e?.message ?? e,
        );
        const sig = await c.sendAndConfirmErTransaction(r.instructions, [
          keypair,
          ...(r.additionalSigners ?? []),
        ]);
        sigs.push({ step: "delegate-basket-er", signature: sig });
      }
    } else {
      // SDK no-op due to MagicBlock ownership on L1. Per the new SDK,
      // `basket.delegate` is a deprecated field (see BasketAccount.deprecatedDelegate)
      // — opens use session keys, not basket.delegate. Setting basket.delegate
      // from ER is rejected by Flash program with Custom:27 (UnsupportedToken)
      // because the user's basket PDA isn't part of the ER pool config. Skip
      // the explicit delegate call entirely.
      sigs.push({
        step: "delegate-basket",
        signature: null,
        skipped:
          "sdk-noop-basket-on-magicblock-delegate-deprecated-opens-use-session-keys",
      });
    }
  }

  const readiness = await readTradeReadiness(c);
  console.error("[wrapper] setup readiness", JSON.stringify(readiness));
  return { signatures: sigs, readiness };
}

function isCapacityError(msg) {
  // Flash on-chain: 6024 CustodyAmountLimit, 6025 PositionAmountLimit,
  // 6032 MaxUtilization, 6088 MaxPositionSize, 6089 MaxExposure, 6110 InsufficientCustodyLiquidity
  return /Custom["']?:\s*60(24|25|32|88|89)|Custom["']?:\s*6110|CustodyAmountLimit|MaxUtilization|MaxPositionSize|MaxExposure|InsufficientCustodyLiquidity/i.test(
    String(msg),
  );
}

function isMinCollateralError(msg) {
  return /Custom["']?:\s*6034|MinCollateral/i.test(String(msg));
}

/** Anchor InstructionFallbackNotFound — program has no matching instruction. */
function isProgramMismatchError(msg) {
  const s = String(msg);
  return (
    /InstructionFallbackNotFound/i.test(s) ||
    /Custom["']?:\s*101\b/.test(s) ||
    /custom program error:\s*0x65/i.test(s) ||
    /Error Number:\s*101/.test(s)
  );
}

function isInsufficientBalanceError(msg) {
  return /Custom["']?:\s*607[89]|InsufficientBalance|InsufficientAvailableBalance/i.test(
    String(msg),
  );
}

function readPositiveIntEnv(name, fallback) {
  const raw = process.env[name];
  if (!raw) return fallback;
  const value = Number.parseInt(raw, 10);
  return Number.isFinite(value) && value > 0 ? value : fallback;
}

async function doOpenPosition(params) {
  const c = await initClient();
  const { collateralAmount, leverage, side: sideStr } = params;
  if (!collateralAmount || !leverage) {
    throw new Error("open_position requires collateralAmount (lamports) and leverage");
  }
  const side = sideStr === "short" || sideStr === "SHORT" ? sideShort() : sideLong();

  // Hard-gate on empty deposit ledger. Empirically (2026-07-22): unfunded
  // HDQ79… always hits OpenPositionEr 6024 CustodyAmountLimit; funded third-party
  // ledgers (e.g. 238 USDC) open successfully on the same market. Soft-continue
  // only burned gas and flooded logs with size backoff.
  const readiness = await readTradeReadiness(c);
  console.error("[wrapper] open readiness", JSON.stringify(readiness));
  if (!readiness.depositOk) {
    throw new Error(
      `wallet not trade-ready: deposit ledger unfunded for SOL. ` +
        `${readiness.issues.join("; ")}. ` +
        `Refusing open to avoid 6024 death spiral.`,
    );
  }
  if (!readiness.basketOk) {
    throw new Error(
      `wallet not trade-ready: basket unreadable on ER. ${readiness.issues.join("; ")}`,
    );
  }

  const { market, lockSymbol, fundingSymbol } = resolveOpenMarket("SOL", side);
  const targetCustody = getCustodyBySymbol("SOL");
  const lockCustody = getCustodyBySymbol(lockSymbol);
  const receivingCustody = getCustodyBySymbol(fundingSymbol);
  // Leverage is fixed from config (e.g. RTP_TRADER_LEVERAGE=9). Do not change it.
  // On pool-capacity errors (6024) only shrink collateral size, not leverage.
  const lev = Number(leverage);
  const levBps = new BN(Math.round(lev * BTC_DECIMALS_BPS));
  let amount = new BN(String(collateralAmount));
  const minAmount = new BN(
    readPositiveIntEnv(
      "RTP_TRADER_MIN_OPEN_COLLATERAL_LAMPORTS",
      DEFAULT_MIN_OPEN_COLLATERAL_LAMPORTS,
    ),
  );
  const maxAttempts = readPositiveIntEnv(
    "RTP_TRADER_OPEN_BACKOFF_ATTEMPTS",
    DEFAULT_OPEN_BACKOFF_ATTEMPTS,
  );
  let lastErr = null;

  console.error(
    `[wrapper] open SOL ${isVariant(side, "long") ? "long" : "short"} market=${market.toBase58?.() ?? market} lockSymbol=${lockSymbol} fundingSymbol=${fundingSymbol} targetCustody=${targetCustody.custodyAccount.toBase58()} lockCustody=${lockCustody.custodyAccount.toBase58()} receivingCustody=${receivingCustody.custodyAccount.toBase58()} amount=${amount.toString()} lev=${lev} minAmount=${minAmount.toString()} maxAttempts=${maxAttempts}`,
  );

  for (let attempt = 0; attempt < maxAttempts; attempt++) {
    if (amount.lt(minAmount)) {
      throw new Error(
        lastErr
          ? `open failed after capacity backoff below ${minAmount.toString()} lamports (lev fixed at ${lev}): ${lastErr}`
          : "collateral amount below minimum after capacity backoff",
      );
    }

    const price = await entryPrice("SOL", side, true);

    // sizeAmount is in SOL base units (target-token), derived from the quote to
    // avoid Custom 6021/6023 (Min/MaxLeverage). leverage is in BPS (BPS_DECIMALS = 4).
    let sizeAmount;
    try {
      const quote = await c.views.getOpenPositionQuoteEr(poolConfig, {
        market,
        targetSymbol: "SOL",
        collateralSymbol: lockSymbol,
        receivingSymbol: fundingSymbol,
        amountIn: amount,
        leverage: levBps,
        owner: keypair.publicKey,
      });
      sizeAmount = quote.sizeAmount;
      console.error(
        `[wrapper] quote attempt=${attempt} market=${market.toBase58?.() ?? market} lockSymbol=${lockSymbol} fundingSymbol=${fundingSymbol} amount=${amount.toString()} lev=${lev} sizeAmount=${sizeAmount?.toString?.() ?? sizeAmount}`,
      );
    } catch (e) {
      const msg = e?.message ?? String(e);
      lastErr = `quote failed: ${msg}`;
      console.error(`[wrapper] ${lastErr}`);
      if (isProgramMismatchError(msg)) {
        throw new Error(
          `open quote failed: Flash program InstructionFallbackNotFound (Custom 101). ` +
            `Deployed ER program does not match SDK IDL (open_position_er / views). Not a capacity issue. Original: ${msg}`,
        );
      }
      if (isCapacityError(msg) && attempt < maxAttempts - 1) {
        amount = amount.div(new BN(2));
        continue;
      }
      throw e;
    }

    try {
      // 2nd arg is lockSymbol; SOL longs resolve to JitoSOL in SDK v2.
      // 3rd arg is the user's funding/receiving custody, kept as SOL.
      const { instructions } = await c.openPosition(
        "SOL",
        lockSymbol,
        fundingSymbol,
        side,
        poolConfig,
        price,
        amount,
        sizeAmount,
      );

      const sig = await c.sendAndConfirmErTransaction(instructions, [keypair]);
      return {
        signature: sig,
        sizeAmount: sizeAmount.toString(),
        collateralAmount: amount.toString(),
        collateralSymbol: lockSymbol,
        fundingSymbol,
        leverage: lev,
        side: isVariant(side, "long") ? "long" : "short",
        attempt,
      };
    } catch (e) {
      const msg = e?.message ?? String(e);
      lastErr = msg;
      if (isProgramMismatchError(msg)) {
        throw new Error(
          `open failed: Flash program InstructionFallbackNotFound (Custom 101). ` +
            `ER program rejected open_position_er — upstream IDL/program mismatch, not CustodyAmountLimit. Original: ${msg}`,
        );
      }
      if (isInsufficientBalanceError(msg)) {
        throw new Error(
          `open failed: insufficient Flash deposit-ledger balance (not pool capacity). ` +
            `Fund via setup/deposit-direct. Original: ${msg}`,
        );
      }
      if (isMinCollateralError(msg)) {
        throw new Error(
          `open failed: Flash rejected ${amount.toString()} lamports as below MinCollateral after capacity backoff (lev fixed at ${lev}): ${msg}`,
        );
      }
      if (isCapacityError(msg) && attempt < maxAttempts - 1) {
        // Halve collateral only — leverage stays at config value.
        amount = amount.div(new BN(2));
        console.error(
          `[wrapper] capacity error on open (attempt ${attempt + 1}/${maxAttempts}): ${msg} — retry amount=${amount.toString()} lev=${lev} (unchanged)`,
        );
        continue;
      }
      throw e;
    }
  }

  throw new Error(lastErr || "open_position failed");
}

async function doClosePosition(params) {
  const c = await initClient();
  const { side: sideStr, collateralSymbol: collateralSymbolParam } = params;
  const side = sideStr === "short" || sideStr === "SHORT" ? sideShort() : sideLong();

  const collateralSymbol = collateralSymbolParam || getMarket("SOL", side).collateralSymbol;
  const { market } = getMarketForLock("SOL", collateralSymbol, side);
  const price = await entryPrice("SOL", side, false);

  console.error(
    `[wrapper] close SOL ${isVariant(side, "long") ? "long" : "short"} market=${market.toBase58?.() ?? market} collateralSymbol=${collateralSymbol}`,
  );

  const { instructions: closeIxs } = await c.closePosition(
    "SOL",
    collateralSymbol,
    side,
    poolConfig,
    price,
  );

  // Cancel any stale TP/SL trigger orders parked on the basket so they don't
  // reopen the position after close. Per docs, the program returns err if we
  // pass cancelIxs on a market without orders — guard with try/catch.
  const allIxs = [...closeIxs];
  try {
    const { instructions: cancelIxs } = await c.cancelAllTriggerOrders(market);
    if (cancelIxs?.length) allIxs.push(...cancelIxs);
  } catch (_) {
    // no trigger orders, or already cancelled — proceed with close only
  }

  const sig = await c.sendAndConfirmErTransaction(allIxs, [keypair]);
  return { signature: sig, collateralSymbol };
}

async function doGetPrice(params) {
  const { symbol = "SOL", side: sideStr = "long", slippageBps = 100 } = params;
  const side = sideStr === "short" || sideStr === "SHORT" ? sideShort() : sideLong();
  const price = await entryPrice(symbol, side, true, slippageBps);
  return {
    price: price.price.toString(),
    exponent: price.exponent,
  };
}

// Stdin/stdout newline-delimited JSON-RPC.
const decoder = new TextDecoder();
let buffer = "";

process.stdin.on("data", (chunk) => {
  buffer += decoder.decode(chunk);
  const lines = buffer.split("\n");
  buffer = lines.pop();
  for (const line of lines) {
    if (!line.trim()) continue;
    let req;
    try {
      req = JSON.parse(line);
    } catch (e) {
      process.stdout.write(
        JSON.stringify({ jsonrpc: "2.0", id: null, error: { code: -32700, message: "Parse error: " + e.message } }) +
          "\n",
      );
      continue;
    }
    handleRequest(req).then((res) => {
      process.stdout.write(JSON.stringify(res) + "\n");
    });
  }
});

process.stdin.on("end", () => {
  if (buffer.trim()) {
    try {
      const req = JSON.parse(buffer);
      handleRequest(req).then((res) => {
        process.stdout.write(JSON.stringify(res) + "\n");
      });
    } catch (_) {
      // ignore trailing garbage
    }
  }
});

// Ready MUST go to stdout — the Rust parent only pipes stdout for the
// ready handshake (stderr is inherited for logs). If this is on stderr,
// the parent times out and kills a healthy wrapper.
process.stdout.write("[wrapper] Ready for JSON-RPC requests\n");
console.error("[wrapper] Ready signal sent on stdout");
