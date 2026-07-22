#!/usr/bin/env node
/**
 * Flash V2: fund deposit ledger with SOL, then open a micro SOL long on ER.
 *
 * Uses production trader keypair only:
 *   ~/.config/solana/rtp-trader.json  → HDQ79fQ1YbL9CenS1DzfHizEWGrJdnmo99fgAWmdhuy5
 *
 * Docs path (source of truth):
 *   deposit / deposit-direct (base Solana RPC) → open-position (ER)
 *   SOL is a valid collateral token; SOL longs lock JitoSOL under the hood.
 *
 * Current mainnet blocker (as of program upgrade slot 434407053 / 2026-07-22T00:33Z):
 *   deposit_direct returns InstructionFallbackNotFound (101) because the
 *   deployed FLASH6 binary no longer includes that instruction. This script
 *   polls until deposit works, then funds and opens.
 *
 * Usage:
 *   node scripts/flash-fund-and-open-sol.mjs
 *   RTP_DEPOSIT_SOL=0.5 RTP_OPEN_SOL=0.05 RTP_LEVERAGE=9 node scripts/flash-fund-and-open-sol.mjs
 *   RTP_POLL_SECS=30 RTP_MAX_ATTEMPTS=60 node scripts/flash-fund-and-open-sol.mjs
 */
import {
  FlashPerpetualsClient,
  PROGRAM_ID,
} from "@flash_trade/flash-sdk-v2";
import { AnchorProvider, Wallet } from "@coral-xyz/anchor";
import {
  Connection,
  Keypair,
  PublicKey,
  VersionedTransaction,
  TransactionMessage,
} from "@solana/web3.js";
import { NATIVE_MINT } from "@solana/spl-token";
import BN from "bn.js";
import fs from "fs";
import os from "os";
import path from "path";

const EXPECTED = "HDQ79fQ1YbL9CenS1DzfHizEWGrJdnmo99fgAWmdhuy5";
const KEYPAIR_PATH =
  process.env.RTP_TRADER_KEYPAIR_PATH ||
  path.join(os.homedir(), ".config/solana/rtp-trader.json");
const RPC = process.env.RTP_SOLANA_RPC_URL || "https://api.mainnet-beta.solana.com";
const ER = process.env.RTP_TRADER_ER_RPC || "https://flash.magicblock.xyz";
const API = process.env.FLASH_API_URL || "https://flashapi.trade";
const PROGRAMDATA = new PublicKey("8ta4NRHQxtYta4w1VqtW9mKDwrnS5F8wRcSJDKLTGjTi");

const DEPOSIT_SOL = Number(process.env.RTP_DEPOSIT_SOL || "0.5");
const OPEN_SOL = Number(process.env.RTP_OPEN_SOL || "0.05");
const LEVERAGE = Number(process.env.RTP_LEVERAGE || "9");
const POLL_SECS = Number(process.env.RTP_POLL_SECS || "30");
const MAX_ATTEMPTS = Number(process.env.RTP_MAX_ATTEMPTS || "60");

function loadKeypair() {
  const secret = JSON.parse(fs.readFileSync(KEYPAIR_PATH, "utf8"));
  const kp = Keypair.fromSecretKey(Uint8Array.from(secret));
  if (kp.publicKey.toBase58() !== EXPECTED) {
    throw new Error(
      `wrong keypair ${kp.publicKey.toBase58()} (expected ${EXPECTED})`,
    );
  }
  return kp;
}

async function programSlot(conn) {
  const info = await conn.getAccountInfo(PROGRAMDATA);
  return info.data.readBigUInt64LE(4).toString();
}

async function depositDirectWorks(client, kp, lamports) {
  const r = await client.depositDirect(NATIVE_MINT, new BN(lamports));
  const conn = client.provider.connection;
  const { blockhash } = await conn.getLatestBlockhash("confirmed");
  const msg = new TransactionMessage({
    payerKey: kp.publicKey,
    recentBlockhash: blockhash,
    instructions: r.instructions,
  }).compileToV0Message();
  const tx = new VersionedTransaction(msg);
  tx.sign([kp, ...(r.additionalSigners || [])]);
  const sim = await conn.simulateTransaction(tx, { sigVerify: true });
  return { sim, tx, r, blockhash };
}

async function sendDeposit(client, kp, lamports) {
  const { sim, tx, blockhash } = await depositDirectWorks(client, kp, lamports);
  if (sim.value.err) {
    const log = (sim.value.logs || []).find((l) =>
      /Error|Fallback|Deposit/.test(l),
    );
    return { ok: false, err: sim.value.err, log };
  }
  const conn = client.provider.connection;
  const { lastValidBlockHeight } = await conn.getLatestBlockhash("confirmed");
  const sig = await conn.sendRawTransaction(tx.serialize(), {
    skipPreflight: false,
  });
  await conn.confirmTransaction(
    { signature: sig, blockhash, lastValidBlockHeight },
    "confirmed",
  );
  return { ok: true, sig };
}

async function openSolLong(kp, amountUi, leverage) {
  const body = {
    inputTokenSymbol: "SOL",
    outputTokenSymbol: "SOL",
    inputAmountUi: String(amountUi),
    leverage: Number(leverage),
    tradeType: "LONG",
    owner: kp.publicKey.toBase58(),
    orderType: "MARKET",
    slippagePercentage: "1.0",
  };
  const res = await fetch(`${API}/transaction-builder/open-position`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  const j = await res.json();
  if (!j.transactionBase64) {
    return { ok: false, stage: "build", body: j };
  }
  const er = new Connection(ER, "confirmed");
  const tx = VersionedTransaction.deserialize(
    Buffer.from(j.transactionBase64, "base64"),
  );
  // Partial-sign owner only — never mutate blockhash on Flash API txs
  tx.sign([kp]);
  const sim = await er.simulateTransaction(tx, { sigVerify: true });
  if (sim.value.err) {
    const log = (sim.value.logs || []).find((l) => /Error Code/.test(l));
    return { ok: false, stage: "sim", err: sim.value.err, log };
  }
  const sig = await er.sendRawTransaction(tx.serialize(), {
    skipPreflight: false,
    maxRetries: 3,
  });
  return {
    ok: true,
    sig,
    quote: {
      leverage: j.newLeverage,
      entry: j.newEntryPrice,
      liq: j.newLiquidationPrice,
    },
  };
}

async function ledgerSummary(client, owner) {
  try {
    const ledger = await client.accounts.fetchUserDepositLedger(owner);
    const deps = (ledger.deposits || []).map((d) => ({
      mint: d.mint?.toBase58?.() || d.mintKey?.toBase58?.(),
      amount: d.amount?.toString?.() || d.balance?.toString?.(),
    }));
    return deps;
  } catch (e) {
    return { error: e.message };
  }
}

async function main() {
  const kp = loadKeypair();
  const base = new Connection(RPC, "confirmed");
  const provider = new AnchorProvider(base, new Wallet(kp), {
    commitment: "confirmed",
  });
  const client = new FlashPerpetualsClient(
    provider,
    undefined,
    PROGRAM_ID["mainnet-beta"],
    { prioritizationFee: 100_000, txConfirmationCommitment: "confirmed" },
    ER,
  );

  console.log("owner", kp.publicKey.toBase58());
  console.log("wallet SOL", (await base.getBalance(kp.publicKey)) / 1e9);
  console.log("program slot", await programSlot(base));
  console.log("ledger before", await ledgerSummary(client, kp.publicKey));

  for (let i = 1; i <= MAX_ATTEMPTS; i++) {
    console.log(`\n=== attempt ${i}/${MAX_ATTEMPTS} ===`);
    console.log("program slot", await programSlot(base));

    const probe = await depositDirectWorks(
      client,
      kp,
      Math.round(0.01 * 1e9),
    );
    if (probe.sim.value.err) {
      const log = (probe.sim.value.logs || []).find((l) =>
        /Error|Fallback|Deposit/.test(l),
      );
      console.log("deposit still failing:", JSON.stringify(probe.sim.value.err));
      console.log(" ", log || "");
      if (i < MAX_ATTEMPTS) {
        await new Promise((r) => setTimeout(r, POLL_SECS * 1000));
      }
      continue;
    }

    console.log("deposit instruction accepted — funding", DEPOSIT_SOL, "SOL");
    const dep = await sendDeposit(
      client,
      kp,
      Math.round(DEPOSIT_SOL * 1e9),
    );
    console.log("deposit", dep);
    if (!dep.ok) {
      console.error("deposit send failed after successful sim");
      process.exit(2);
    }

    console.log("ledger after deposit", await ledgerSummary(client, kp.publicKey));
    console.log("opening SOL long", OPEN_SOL, "SOL @", LEVERAGE, "x");
    const open = await openSolLong(kp, OPEN_SOL, LEVERAGE);
    console.log("open", open);
    if (!open.ok) process.exit(3);
    console.log("SUCCESS open sig", open.sig);
    process.exit(0);
  }

  console.error(
    "Gave up: deposit_direct still InstructionFallbackNotFound (101). " +
      "Flash mainnet FLASH6 binary (post slot 434407053) is missing deposit_direct. " +
      "Opens require a funded deposit ledger; SOL collateral is correct once funded.",
  );
  process.exit(1);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
