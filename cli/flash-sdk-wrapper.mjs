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
      default:
        throw new Error(`Unknown method: ${method}`);
    }
    return { jsonrpc: "2.0", id, result };
  } catch (e) {
    return { jsonrpc: "2.0", id, error: { code: -32000, message: e?.message ?? String(e) } };
  }
}

// Resolve the market for (targetSymbol, side) and return { marketPk, side, collateralSymbol }.
// Mirrors the docs "Resolving the market" snippet: collateral is derived from the
// market PDA — never hardcoded — to avoid ConstraintSeeds (Custom 2006).
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

// Five-step v2 setup. Steps 1, 2, 3, 5 are idempotent. Step 4 (depositDirect) is
// non-idempotent (transfers value), so we skip it when the deposit ledger already
// holds a usable balance. Caller has wallet pre-funded at 2.4 SOL per operations.
async function doSetup() {
  const c = await initClient();
  const sigs = [];

  async function runStep(name, builder) {
    let r;
    try {
      r = await builder();
    } catch (e) {
      // already-initialized races surface as zero-instruction results on the
      // happy path, but if the SDK throws first we still treat "already
      // initialized" as a no-op.
      const msg = e?.message ?? String(e);
      if (/already/i.test(msg) || /initialized/i.test(msg)) {
        sigs.push({ step: name, signature: null, skipped: "already-initialized" });
        return;
      }
      throw e;
    }
    if (!r || !Array.isArray(r.instructions) || r.instructions.length === 0) {
      sigs.push({ step: name, signature: null, skipped: "noop" });
      return;
    }
    const sig = await c.sendAndConfirmErTransaction(r.instructions, [
      keypair,
      ...(r.additionalSigners ?? []),
    ]);
    sigs.push({ step: name, signature: sig });
  }

  await runStep("init-deposit-ledger", () => c.initializeUserDepositLedger());
  await runStep("init-basket", () => c.initializeBasket());
  await runStep("init-trade-vault-SOL", () => c.initTradeVault(SOL_MINT));

  // Skip deposit if the ledger is already funded. fetchUserDepositLedger returns
  // any per-token balances the program tracks; if SOL ledger already has enough
  // balance we don't run depositDirect (which transfers lamports on each call).
  let skipDeposit = false;
  try {
    const ledger = await c.accounts.fetchUserDepositLedger(keypair.publicKey);
    // The ledger shape exposes per-token entries; check for SOL entry with
    // balance >= threshold. Field access is best-effort — if the SDK shape
    // changes, fall through to deposit (safer than double-depositing).
    const entries = ledger?.deposits ?? ledger?.entries ?? [];
    const solEntry = entries.find((d) => {
      const mint = d?.mintKey ?? d?.tokenMint ?? d?.mint;
      return mint && new PublicKey(mint).equals?.(SOL_MINT);
    });
    const bal = solEntry?.balance ?? solEntry?.amount ?? solEntry?.lamports;
    if (bal instanceof BN && bal.gte(new BN(DEPOSIT_SKIP_FUNDED_LAMPORTS))) {
      skipDeposit = true;
    }
  } catch (e) {
    console.error("[wrapper] ledger read failed, will run depositDirect:", e?.message);
  }

  if (skipDeposit) {
    sigs.push({
      step: "deposit-direct",
      signature: null,
      skipped: `ledger-funded (>=${DEPOSIT_SKIP_FUNDED_LAMPORTS} lamports)`,
    });
  } else {
    await runStep("deposit-direct", () => c.depositDirect(SOL_MINT, new BN(1_000_000_000)));
  }

  await runStep("delegate-basket", () => c.delegateBasket(keypair.publicKey));

  return { signatures: sigs };
}

async function doOpenPosition(params) {
  const c = await initClient();
  const { collateralAmount, leverage, side: sideStr } = params;
  if (!collateralAmount || !leverage) {
    throw new Error("open_position requires collateralAmount (lamports) and leverage");
  }
  const side = sideStr === "short" || sideStr === "SHORT" ? sideShort() : sideLong();

  const { market, collateralSymbol } = getMarket("SOL", side);
  const price = await entryPrice("SOL", side, true);

  // sizeAmount is in SOL base units (target-token), derived from the quote to
  // avoid Custom 6021/6023 (Min/MaxLeverage). leverage is in BPS (BPS_DECIMALS = 4).
  const { sizeAmount } = await c.views.getOpenPositionQuoteEr(poolConfig, {
    market,
    targetSymbol: "SOL",
    collateralSymbol,
    receivingSymbol: collateralSymbol,
    amountIn: new BN(collateralAmount),
    leverage: new BN(Math.round(leverage * BTC_DECIMALS_BPS)),
  });

  const { instructions } = await c.openPosition(
    "SOL",
    collateralSymbol,
    collateralSymbol,
    side,
    poolConfig,
    price,
    new BN(collateralAmount),
    sizeAmount,
  );

  const sig = await c.sendAndConfirmErTransaction(instructions, [keypair]);
  return {
    signature: sig,
    sizeAmount: sizeAmount.toString(),
    collateralSymbol,
    side: isVariant(side, "long") ? "long" : "short",
  };
}

async function doClosePosition(params) {
  const c = await initClient();
  const { side: sideStr } = params;
  const side = sideStr === "short" || sideStr === "SHORT" ? sideShort() : sideLong();

  const { market, collateralSymbol } = getMarket("SOL", side);
  const price = await entryPrice("SOL", side, false);

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
