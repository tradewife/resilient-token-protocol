// Browser globals
const fetch = (globalThis as any).fetch;
const AbortSignal = (globalThis as any).AbortSignal;

// @resilient-protocol/sdk
// The launchpad integration SDK for the Resilient Token Protocol.
// Core functions: registerWithRTP, fetchTreasuryState, depositSol, checkRedistribute.

import { AnchorProvider, BorshCoder, Program, Idl } from "@coral-xyz/anchor";
import { BN } from "@coral-xyz/anchor";

import {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
  VersionedTransaction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";

// Constants

/** The RTP treasury Anchor program (deployed on devnet). */
export const RTP_PROGRAM_ID = new PublicKey(
  "8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB",
);

/** Devnet RPC endpoint. */
export const RTP_DEVNET_RPC = "https://api.devnet.solana.com";

/** Mainnet RPC endpoint. */
export const RTP_MAINNET_RPC = "https://api.mainnet-beta.solana.com";

// PDA Seeds

const SEED_TREASURY = Buffer.from("treasury");
const SEED_ADOPTER = Buffer.from("adopter");

function deriveTreasuryPDA(authority: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEED_TREASURY, authority.toBuffer()],
    RTP_PROGRAM_ID,
  );
}

function deriveAdopterPDA(treasury: PublicKey, adopterId: string): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEED_ADOPTER, treasury.toBuffer(), Buffer.from(adopterId)],
    RTP_PROGRAM_ID,
  );
}

// IDL (bundled inline — no runtime file dependency)

import { IDL } from "./idl";

function loadPatchedIdl(): Idl {
  const idl = JSON.parse(JSON.stringify(IDL));
  idl.address = RTP_PROGRAM_ID.toBase58();
  // Anchor 0.31 workaround: accounts array entries are missing 'type'
  const typeMap: Record<string, any> = {};
  for (const t of idl.types || []) typeMap[t.name] = t;
  for (const acc of idl.accounts || []) {
    if (!acc.type && typeMap[acc.name]) acc.type = typeMap[acc.name].type;
  }
  return idl;
}

// Types

export interface RTPRegistrationConfig {
  /** The authority that owns the treasury (used as PDA seed) */
  authority: PublicKey;
  /** Wallet receiving 70% holder distributions */
  holdersWallet?: PublicKey;
  /** Wallet receiving 20% dev distributions */
  projectDevWallet?: PublicKey;
  /** Wallet receiving 10% ecosystem distributions */
  ecosystemWallet?: PublicKey;
  /** Minimum runway balance in lamports (default: 10_000_000) */
  minRunwayBalance?: number;
}

export interface RTPRegistrationResult {
  /** The authority pubkey (base58) */
  authority: string;
  /** Transaction signature of the initialization */
  signature: string;
  /** Solana Explorer link */
  explorerUrl: string;
  /** Treasury state account */
  treasuryPDA: string;
  /** Adopter registration account */
  adopterPDA: string;
}

/** Minimal wallet adapter interface — compatible with @solana/wallet-adapter-react.
 *  Supports both legacy Transaction and VersionedTransaction (required for
 *  Pump.fun and other platforms that return VersionedTransaction from APIs). */
export interface WalletAdapter {
  publicKey: PublicKey | null;
  signTransaction<T extends Transaction | VersionedTransaction>(tx: T): Promise<T>;
}

export interface TreasuryState {
  authority: string;
  phase: "Sustenance" | "Ecosystem" | "Humanity";
  solBalance: number;          // native SOL lamports in treasury
  committedSolLamports: number; // committed to open Flash positions
  availableSolLamports: number; // solBalance - committed - rent_exempt
  totalFeesWithdrawn: number;
  totalDistributedHolders: number;
  totalDistributedDev: number;
  totalDistributedEcosystem: number;
  totalHydration: number;
  totalFeesReceived: number;
  minRunwayBalance: number;
  /** Whether the treasury is frozen (emergency halt). */
  isFrozen: boolean;
}

/** Check if a signer is a Keypair (works across ESM/CJS module boundaries).
 *  Uses duck-typing: checks for `secretKey` as a byte-like object.
 *  Avoids `instanceof Uint8Array` which fails when the Keypair class is loaded
 *  from a different module realm (e.g., ESM tsx vs CJS node_modules). */
function isKeypair(payer: Keypair | WalletAdapter): payer is Keypair {
  if (!("secretKey" in payer)) return false;
  const sk = (payer as unknown as Record<string, unknown>).secretKey;
  // Duck-type: must be a typed array or Buffer with byte length > 0
  return (typeof sk === "object" && sk !== null &&
    ("byteLength" in (sk as object) || "length" in (sk as object)));
}

/** Wrap a Keypair as a minimal wallet for AnchorProvider (avoids anchor.Wallet ESM issue). */
function kpWallet(kp: Keypair) {
  return {
    publicKey: kp.publicKey,
    signTransaction: async <T extends Transaction | VersionedTransaction>(tx: T): Promise<T> => {
      if (tx instanceof VersionedTransaction) {
        tx.sign([kp]);
      } else {
        tx.partialSign(kp);
      }
      return tx;
    },
    signAllTransactions: async <T extends Transaction | VersionedTransaction>(txs: T[]): Promise<T[]> => {
      return txs.map(tx => {
        if (tx instanceof VersionedTransaction) {
          tx.sign([kp]);
        } else {
          tx.partialSign(kp);
        }
        return tx;
      });
    },
  };
}

/** Adapt a WalletAdapter into the wallet interface expected by AnchorProvider. */
function walletToAnchorWallet(adapter: WalletAdapter) {
  return {
    publicKey: adapter.publicKey!,
    signTransaction: async <T extends Transaction | VersionedTransaction>(tx: T): Promise<T> => {
      return adapter.signTransaction(tx);
    },
    signAllTransactions: async <T extends Transaction | VersionedTransaction>(txs: T[]): Promise<T[]> => {
      return Promise.all(txs.map(tx => adapter.signTransaction(tx)));
    },
  };
}

// Helpers

/** Send a pre-signed transaction via sendRawTransaction (correct for WalletAdapter). */
async function sendSignedTx(
  connection: Connection,
  signedTx: Transaction,
): Promise<string> {
  const rawTx = signedTx.serialize();
  const sig = await connection.sendRawTransaction(rawTx);
  await connection.confirmTransaction(
    {
      signature: sig,
      blockhash: signedTx.recentBlockhash!,
      lastValidBlockHeight: signedTx.lastValidBlockHeight!,
    },
    "confirmed",
  );
  return sig;
}

/** Send a transaction, handling both Keypair and WalletAdapter signers. */
async function sendTx(
  connection: Connection,
  tx: Transaction,
  payer: Keypair | WalletAdapter,
  extraSigners: Keypair[] = [],
): Promise<string> {
  const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
  tx.recentBlockhash = blockhash;
  tx.lastValidBlockHeight = lastValidBlockHeight;
  tx.feePayer = isKeypair(payer) ? payer.publicKey : payer.publicKey!;

  if (isKeypair(payer)) {
    return sendAndConfirmTransaction(connection, tx, [payer, ...extraSigners]);
  } else {
    // Belt-and-suspenders: if isKeypair returned false but secretKey is present,
    // the object is a Keypair from a different module realm (e.g., dynamic ESM
    // import via tsx). Reconstruct and use the keypair signing path to avoid
    // calling WalletAdapter.signTransaction on a Keypair object.
    const maybeSecret = (payer as unknown as Record<string, unknown>).secretKey;
    if (maybeSecret && typeof maybeSecret === "object" &&
        ("byteLength" in (maybeSecret as object) || "length" in (maybeSecret as object))) {
      const kp = Keypair.fromSecretKey(maybeSecret as Uint8Array);
      return sendAndConfirmTransaction(connection, tx, [kp, ...extraSigners]);
    }
    if (extraSigners.length > 0) tx.partialSign(...extraSigners);
    const signed = await payer.signTransaction(tx);
    return sendSignedTx(connection, signed);
  }
}

// Implementation

/**
 * Initialize a new Treasury with the given authority.
 * Creates the treasury PDA (authority-seeded) and registers the first adopter.
 *
 * This is the single integration point for launchpads — one call per project.
 */
export async function registerWithRTP(
  connection: Connection,
  payer: Keypair | WalletAdapter,
  config: RTPRegistrationConfig,
): Promise<RTPRegistrationResult> {
  const payerPubkey = isKeypair(payer) ? payer.publicKey : payer.publicKey!;
  const authority = config.authority;

  // Derive authority-seeded treasury PDA
  const [treasuryPDA] = deriveTreasuryPDA(authority);

  const holdersWallet = config.holdersWallet ?? payerPubkey;
  const projectDevWallet = config.projectDevWallet ?? payerPubkey;
  const ecosystemWallet = config.ecosystemWallet ?? payerPubkey;
  const minRunwayBalance = config.minRunwayBalance ?? 10_000_000;

  // Default adopter_id = authority.toBase58() (any string works, this is backwards-compatible)
  const adopterId = authority.toBase58();
  const [adopterPDA] = deriveAdopterPDA(treasuryPDA, adopterId);

  const idl = loadPatchedIdl();
  const provider = new AnchorProvider(
    connection,
    isKeypair(payer) ? kpWallet(payer) : walletToAnchorWallet(payer),
    { commitment: "confirmed" },
  );
  const program = new Program(idl, provider);

  // Step 1: Initialize treasury (authority-seeded, no mint/vault)
  const initTx = await program.methods
    .initialize(holdersWallet, projectDevWallet, ecosystemWallet, new BN(minRunwayBalance))
    .accounts({
      treasury: treasuryPDA,
      authority: payerPubkey,
      systemProgram: SystemProgram.programId,
    })
    .transaction();

  const signature = await sendTx(connection, initTx, payer);

  // Step 2: Register the first adopter (adopterId = authority as string)
  try {
    const adopterTx = await program.methods
      .registerAdopter(adopterId)
      .accounts({
        adopterRecord: adopterPDA,
        treasury: treasuryPDA,
        authority: payerPubkey,
        systemProgram: SystemProgram.programId,
      })
      .transaction();

    await sendTx(connection, adopterTx, payer);
  } catch (e: unknown) {
    console.warn("[RTP SDK] Adopter registration skipped:", e instanceof Error ? e.message : String(e));
  }

  const cluster = connection.rpcEndpoint.includes("devnet") ? "devnet" : "mainnet-beta";

  return {
    authority: authority.toBase58(),
    signature,
    explorerUrl: `https://explorer.solana.com/tx/${signature}?cluster=${cluster}`,
    treasuryPDA: treasuryPDA.toBase58(),
    adopterPDA: adopterPDA.toBase58(),
  };
}

/**
 * Fetch the on-chain treasury state for a given authority.
 * Read-only — no transactions, no signing required.
 * Returns zeros if the treasury account doesn't exist yet.
 */
export async function fetchTreasuryState(
  connection: Connection,
  authorityAddress: string | PublicKey,
): Promise<TreasuryState> {
  const authority = typeof authorityAddress === "string" ? new PublicKey(authorityAddress) : authorityAddress;
  const [treasuryPDA] = deriveTreasuryPDA(authority);

  const idl = loadPatchedIdl();
  const coder = new BorshCoder(idl);

  // Fetch treasury lamport balance directly
  const treasuryLamports = await connection.getBalance(treasuryPDA);

  // Fetch treasury account data
  const accountInfo = await connection.getAccountInfo(treasuryPDA);
  if (!accountInfo) {
    return {
      authority: authority.toBase58(),
      phase: "Sustenance",
      solBalance: 0,
      committedSolLamports: 0,
      availableSolLamports: 0,
      totalFeesWithdrawn: 0,
      totalDistributedHolders: 0,
      totalDistributedDev: 0,
      totalDistributedEcosystem: 0,
      totalHydration: 0,
      totalFeesReceived: 0,
      minRunwayBalance: 0,
      isFrozen: false,
    };
  }

  const treasury = coder.accounts.decode("Treasury", accountInfo.data);

  // Decode phase enum
  const phaseKey = Object.keys(treasury.phase)[0];
  const phase = (phaseKey.charAt(0).toUpperCase() + phaseKey.slice(1)) as TreasuryState["phase"];

  // Rent-exempt minimum for treasury account (8 + INIT_SPACE)
  const RENT_EXEMPT_MINIMUM = await connection.getMinimumBalanceForRentExemption(8 + 324); // Approximate INIT_SPACE

  const solBalance = treasuryLamports;
  const committedSolLamports = Number(treasury.committedSolLamports);
  const availableSolLamports = Math.max(0, solBalance - RENT_EXEMPT_MINIMUM - committedSolLamports);

  return {
    authority: authority.toBase58(),
    phase,
    solBalance,
    committedSolLamports,
    availableSolLamports,
    totalFeesWithdrawn: Number(treasury.totalFeesWithdrawn),
    totalDistributedHolders: Number(treasury.totalDistributedHolders),
    totalDistributedDev: Number(treasury.totalDistributedDev),
    totalDistributedEcosystem: Number(treasury.totalDistributedEcosystem),
    totalHydration: Number(treasury.totalHydration),
    totalFeesReceived: Number(treasury.totalFeesReceivedLamports),
    minRunwayBalance: Number(treasury.minRunwayBalance),
    isFrozen: Boolean(treasury.frozen),
  };
}

/**
 * Deposit native SOL into the treasury.
 * Permissionless — anyone can deposit on behalf of a treasury.
 */
export async function depositSol(
  connection: Connection,
  payer: Keypair | WalletAdapter,
  authorityAddress: string | PublicKey,
  amountLamports: number,
): Promise<{ signature: string }> {
  const authority = typeof authorityAddress === "string" ? new PublicKey(authorityAddress) : authorityAddress;
  const [treasuryPDA] = deriveTreasuryPDA(authority);

  const idl = loadPatchedIdl();
  const provider = new AnchorProvider(
    connection,
    isKeypair(payer) ? kpWallet(payer) : walletToAnchorWallet(payer),
    { commitment: "confirmed" },
  );
  const program = new Program(idl, provider);

  const depositTx = await program.methods
    .depositSol(new BN(amountLamports))
    .accounts({
      treasury: treasuryPDA,
      payer: isKeypair(payer) ? payer.publicKey : (payer as WalletAdapter).publicKey!,
      systemProgram: SystemProgram.programId,
    })
    .transaction();

  const signature = await sendTx(connection, depositTx, payer);
  return { signature };
}

/**
 * Permissionless crank: check redistribution threshold and execute 70/20/10 split.
 * Callable by anyone. No pre-step needed (no mint withdrawal).
 */
export async function checkRedistribute(
  connection: Connection,
  payer: Keypair | WalletAdapter,
  authorityAddress: string | PublicKey,
): Promise<{ redistributeSig?: string }> {
  const authority = typeof authorityAddress === "string" ? new PublicKey(authorityAddress) : authorityAddress;
  const [treasuryPDA] = deriveTreasuryPDA(authority);

  const idl = loadPatchedIdl();
  const provider = new AnchorProvider(
    connection,
    isKeypair(payer) ? kpWallet(payer) : walletToAnchorWallet(payer),
    { commitment: "confirmed" },
  );
  const program = new Program(idl, provider);

  try {
    const redistributeTx = await program.methods
      .checkRedistribute()
      .accounts({
        treasury: treasuryPDA,
        holdersWallet: treasuryPDA,
        projectDevWallet: treasuryPDA,
        ecosystemWallet: treasuryPDA,
        systemProgram: SystemProgram.programId,
      })
      .transaction();

    const redistributeSig = await sendTx(connection, redistributeTx, payer);
    return { redistributeSig };
  } catch (err: unknown) {
    const anchorErr = err as {
      error?: { errorCode?: { code?: string; number?: number } };
      message?: string;
    };
    const isBelowThreshold =
      anchorErr.error?.errorCode?.code === "BelowThreshold" ||
      anchorErr.message?.includes("BelowThreshold") ||
      anchorErr.error?.errorCode?.number === 6000;

    if (isBelowThreshold) {
      return {};
    }
    throw err;
  }
}


/**
 * Register a beta adopter with an automatic expiry timestamp.
 * After the expiry, hydrate_swarm refuses to fund strategies for this adopter.
 *
 * @param connection - Solana RPC connection
 * @param payer - Keypair or WalletAdapter signing the transaction
 * @param mintAddress - The token mint to register as a beta adopter
 * @param betaExpiresAt - Unix timestamp when the beta expires (must be in the future)
 */

export interface AdopterState {
  treasury: string;
  adopterId: string;
  feesContributed: number;
  adoptedAt: number;
  lastDepositAt: number;
  depositCount: number;
  betaExpiresAt: number;  // 0 = permanent, >0 = unix timestamp
  betaEnded: boolean;
  isBeta: boolean;
}

export async function registerAdopterBeta(
  connection: Connection,
  payer: Keypair | WalletAdapter,
  authorityAddress: string | PublicKey,
  adopterId: string,
  betaExpiresAt: number,
): Promise<{ signature: string; adopterPDA: string }> {
  const authority = typeof authorityAddress === "string" ? new PublicKey(authorityAddress) : authorityAddress;
  const [treasuryPDA] = deriveTreasuryPDA(authority);
  const [adopterPDA] = deriveAdopterPDA(treasuryPDA, adopterId);
  const payerPubkey = isKeypair(payer) ? payer.publicKey : payer.publicKey!;

  const idl = loadPatchedIdl();
  const provider = new AnchorProvider(
    connection,
    isKeypair(payer) ? kpWallet(payer) : walletToAnchorWallet(payer),
    { commitment: "confirmed" },
  );
  const program = new Program(idl, provider);

  const tx = await program.methods
    .registerAdopterBeta(adopterId, new BN(betaExpiresAt))
    .accounts({
      adopterRecord: adopterPDA,
      treasury: treasuryPDA,
      authority: payerPubkey,
      systemProgram: SystemProgram.programId,
    })
    .transaction();

  const signature = await sendTx(connection, tx, payer);

  return { signature, adopterPDA: adopterPDA.toBase58() };
}

/**
 * End a beta adopter's RTP participation early.
 * Only callable by the treasury authority. Sets beta_ended = true.
 * Yield already generated stays with the project.
 *
 * @param connection - Solana RPC connection
 * @param payer - Keypair or WalletAdapter (must be treasury authority)
 * @param authorityAddress - The treasury authority pubkey
 * @param adopterId - The adopter identifier whose beta to end
 */
export async function endBeta(
  connection: Connection,
  payer: Keypair | WalletAdapter,
  authorityAddress: string | PublicKey,
  adopterId: string,
): Promise<{ signature: string }> {
  const authority = typeof authorityAddress === "string" ? new PublicKey(authorityAddress) : authorityAddress;
  const [treasuryPDA] = deriveTreasuryPDA(authority);
  const [adopterPDA] = deriveAdopterPDA(treasuryPDA, adopterId);

  const idl = loadPatchedIdl();
  const provider = new AnchorProvider(
    connection,
    isKeypair(payer) ? kpWallet(payer) : walletToAnchorWallet(payer),
    { commitment: "confirmed" },
  );
  const program = new Program(idl, provider);

  const tx = await program.methods
    .endBeta()
    .accounts({
      adopterRecord: adopterPDA,
      treasury: treasuryPDA,
      authority: provider.wallet.publicKey,
    })
    .transaction();

  const signature = await sendTx(connection, tx, payer);

  return { signature };
}

/**
 * Fetch the on-chain adopter record for a given mint.
 * Read-only — no transactions, no signing required.
 * Returns default state if the adopter record doesn't exist.
 */
export async function fetchAdopterState(
  connection: Connection,
  authorityAddress: string | PublicKey,
  adopterId: string,
): Promise<AdopterState> {
  const authority = typeof authorityAddress === "string" ? new PublicKey(authorityAddress) : authorityAddress;
  const [treasuryPDA] = deriveTreasuryPDA(authority);
  const [adopterPDA] = deriveAdopterPDA(treasuryPDA, adopterId);

  const idl = loadPatchedIdl();
  const coder = new BorshCoder(idl);

  const accountInfo = await connection.getAccountInfo(adopterPDA);
  if (!accountInfo) {
    return {
      treasury: treasuryPDA.toBase58(),
      adopterId,
      feesContributed: 0,
      adoptedAt: 0,
      lastDepositAt: 0,
      depositCount: 0,
      betaExpiresAt: 0,
      betaEnded: false,
      isBeta: false,
    };
  }

  const adopter = coder.accounts.decode("AdopterRecord", accountInfo.data);

  return {
    treasury: adopter.treasury.toBase58(),
    adopterId: adopter.adopterId,
    feesContributed: Number(adopter.feesContributedLamports),
    adoptedAt: Number(adopter.adoptedAt),
    lastDepositAt: Number(adopter.lastDepositTs),
    depositCount: Number(adopter.depositCount),
    betaExpiresAt: Number(adopter.betaExpiresAt),
    betaEnded: adopter.betaEnded,
    isBeta: Number(adopter.betaExpiresAt) > 0,
  };
}

// ---------------------------------------------------------------------------
// Emergency Freeze / Unfreeze
// ---------------------------------------------------------------------------

/**
 * Emergency freeze: halts all treasury operations.
 * Only callable by the treasury authority (Squads multisig in production).
 * No time lock on freeze — emergency speed.
 */
export async function freezeTreasury(
  connection: Connection,
  payer: Keypair | WalletAdapter,
  authorityAddress: string | PublicKey,
): Promise<{ signature: string }> {
  const authority = typeof authorityAddress === "string" ? new PublicKey(authorityAddress) : authorityAddress;
  const [treasuryPDA] = deriveTreasuryPDA(authority);
  const payerPubkey = isKeypair(payer) ? payer.publicKey : payer.publicKey!;

  const idl = loadPatchedIdl();
  const provider = new AnchorProvider(
    connection,
    isKeypair(payer) ? kpWallet(payer) : walletToAnchorWallet(payer),
    { commitment: "confirmed" },
  );
  const program = new Program(idl, provider);

  const tx = await program.methods
    .freezeTreasury()
    .accounts({
      treasury: treasuryPDA,
      authority: payerPubkey,
    })
    .transaction();

  const signature = await sendTx(connection, tx, payer);
  return { signature };
}

/**
 * Unfreeze: resumes treasury operations.
 * Only callable by the treasury authority (Squads multisig in production).
 * In production, requires 2-of-3 + 24h time lock via Squads proposal.
 */
export async function unfreezeTreasury(
  connection: Connection,
  payer: Keypair | WalletAdapter,
  authorityAddress: string | PublicKey,
): Promise<{ signature: string }> {
  const authority = typeof authorityAddress === "string" ? new PublicKey(authorityAddress) : authorityAddress;
  const [treasuryPDA] = deriveTreasuryPDA(authority);
  const payerPubkey = isKeypair(payer) ? payer.publicKey : payer.publicKey!;

  const idl = loadPatchedIdl();
  const provider = new AnchorProvider(
    connection,
    isKeypair(payer) ? kpWallet(payer) : walletToAnchorWallet(payer),
    { commitment: "confirmed" },
  );
  const program = new Program(idl, provider);

  const tx = await program.methods
    .unfreezeTreasury()
    .accounts({
      treasury: treasuryPDA,
      authority: payerPubkey,
    })
    .transaction();

  const signature = await sendTx(connection, tx, payer);
  return { signature };
}

/**
 * Check whether a treasury is currently frozen.
 * Read-only — no transactions, no signing required.
 */
export async function isTreasuryFrozen(
  connection: Connection,
  authorityAddress: string | PublicKey,
): Promise<boolean> {
  const state = await fetchTreasuryState(connection, authorityAddress);
  return state.isFrozen;
}

// ---------------------------------------------------------------------------
// Strategy Promotion
// ---------------------------------------------------------------------------

/** Derive the strategy record PDA for a given treasury + strategy_id. */
function deriveStrategyPDA(treasury: PublicKey, strategyId: string): [PublicKey, number] {
  const SEED_STRATEGY = Buffer.from("strategy");
  return PublicKey.findProgramAddressSync(
    [SEED_STRATEGY, treasury.toBuffer(), Buffer.from(strategyId)],
    RTP_PROGRAM_ID,
  );
}

/**
 * Register (promote) a strategy to Live status on-chain.
 * Only callable by the treasury authority.
 *
 * @param connection - Solana RPC connection
 * @param payer - Keypair or WalletAdapter (must be treasury authority)
 * @param mintAddress - The token mint for the treasury
 * @param strategyId - Short strategy identifier (1-16 chars, e.g. "S01", "SOL_MR_v2")
 * @param promotionSharpeX100 - OOS Sharpe * 100 (e.g. 396 for Sharpe 3.96)
 */
export async function registerStrategy(
  connection: Connection,
  payer: Keypair | WalletAdapter,
  authorityAddress: string | PublicKey,
  strategyId: string,
  promotionSharpeX100: number,
): Promise<{ signature: string; strategyPDA: string }> {
  const authority = typeof authorityAddress === "string" ? new PublicKey(authorityAddress) : authorityAddress;
  const [treasuryPDA] = deriveTreasuryPDA(authority);
  const [strategyPDA] = deriveStrategyPDA(treasuryPDA, strategyId);
  const payerPubkey = isKeypair(payer) ? payer.publicKey : payer.publicKey!;

  const idl = loadPatchedIdl();
  const provider = new AnchorProvider(
    connection,
    isKeypair(payer) ? kpWallet(payer) : walletToAnchorWallet(payer),
    { commitment: "confirmed" },
  );
  const program = new Program(idl, provider);

  const tx = await program.methods
    .registerStrategy(strategyId, promotionSharpeX100)
    .accounts({
      treasury: treasuryPDA,
      strategyRecord: strategyPDA,
      authority: payerPubkey,
      systemProgram: SystemProgram.programId,
    })
    .transaction();

  const signature = await sendTx(connection, tx, payer);
  return { signature, strategyPDA: strategyPDA.toBase58() };
}

// ---------------------------------------------------------------------------
// Emergency / Positions Functions (P1.4)
// ---------------------------------------------------------------------------

/**
 * Result of querying open Flash Trade positions for a treasury.
 */
export interface FlashPositionInfo {
  position_address: string;
  side: "Long" | "Short";
  size_usd: number;
  entry_price: number;
  unrealized_pnl?: number;
  created_at: string; // ISO 8601
  market: string;
}

/**
 * Fetch open Flash Trade positions for a treasury PDA.
 * Uses the Flash Trade REST API. Returns empty array if no positions or API unavailable.
 *
 * Note: This requires knowing the treasury's position addresses, which can be
 * obtained by querying Flash Trade's position list endpoint with the treasury address.
 */
export async function listFlashPositions(
  treasuryAddress: string,
  rpcUrl: string = RTP_DEVNET_RPC,
): Promise<FlashPositionInfo[]> {
  try {
    const response = await fetch(
      `https://flashapi.trade/api/v1/positions?owner=${treasuryAddress}`,
      { signal: AbortSignal.timeout(5000) },
    );
    if (!response.ok) {
      console.warn(`[listFlashPositions] Flash API returned ${response.status}`);
      return [];
    }
    const data = await response.json() as { positions?: FlashPositionInfo[] };
    return data.positions ?? [];
  } catch (err) {
    console.warn(`[listFlashPositions] Flash API unavailable: ${err}`);
    return [];
  }
}

export interface CloseFlashPositionResult {
  signature: string;
  positionAddress: string;
}

/**
 * Close a single Flash Trade perpetual position via `close_flash_position` CPI.
 *
 * Authority-gated. Treasury must not be frozen. Strategy can be Suspended (exiting is always safe).
 *
 * The `flashAccounts` parameter should contain the pre-derived Flash Trade program
 * accounts for the specific market/position being closed. Derive these offline using
 * the same PDA seeds as the open instruction, or use the positions list to retrieve them.
 */
export async function closeFlashPosition(
  connection: Connection,
  payer: Keypair | WalletAdapter,
  authorityAddress: string | PublicKey,
  positionAddress: string,
  flashAccounts: {
    owner: string;
    feePayer: string;
    receivingAccount: string;
    transferAuthority: string;
    perpetuals: string;
    pool: string;
    market: string;
    targetCustody: string;
    targetOracle: string;
    collateralCustody: string;
    collateralOracle: string;
    collateralCustodyTokenAccount: string;
    tokenProgram: string;
    eventAuthority: string;
    flashProgram: string;
    ixSysvar: string;
    collateralMint: string;
  },
  /** Oracle price for close (fetch from Flash Trade API or Pyth). Defaults to 0 (program fetches). */
  oraclePrice?: number,
  /** Slippage tolerance in basis points. Default 500 (5%). */
  slippageBps: number = 500,
): Promise<CloseFlashPositionResult> {
  const authority = typeof authorityAddress === "string" ? new PublicKey(authorityAddress) : authorityAddress;
  const [treasuryPDA] = deriveTreasuryPDA(authority);
  const [strategyPDA] = deriveStrategyPDA(treasuryPDA, "SOL_2.69"); // Use existing strategy PDA

  const idl = loadPatchedIdl();
  const provider = new AnchorProvider(
    connection,
    isKeypair(payer) ? kpWallet(payer) : walletToAnchorWallet(payer),
    { commitment: "confirmed" },
  );
  const program = new Program(idl, provider);

  // Build remaining accounts array (Flash Trade close_position expects 18 accounts)
  const remainingAccounts = [
    // 0: owner (treasury PDA, signer)
    { pubkey: new PublicKey(flashAccounts.owner), isSigner: true, isWritable: false },
    // 1: fee_payer
    { pubkey: new PublicKey(flashAccounts.feePayer), isSigner: true, isWritable: true },
    // 2: receiving_account
    { pubkey: new PublicKey(flashAccounts.receivingAccount), isSigner: false, isWritable: true },
    // 3: transfer_authority
    { pubkey: new PublicKey(flashAccounts.transferAuthority), isSigner: false, isWritable: false },
    // 4: perpetuals
    { pubkey: new PublicKey(flashAccounts.perpetuals), isSigner: false, isWritable: false },
    // 5: pool
    { pubkey: new PublicKey(flashAccounts.pool), isSigner: false, isWritable: true },
    // 6: position
    { pubkey: new PublicKey(positionAddress), isSigner: false, isWritable: true },
    // 7: market
    { pubkey: new PublicKey(flashAccounts.market), isSigner: false, isWritable: true },
    // 8: target_custody
    { pubkey: new PublicKey(flashAccounts.targetCustody), isSigner: false, isWritable: false },
    // 9: target_oracle_account
    { pubkey: new PublicKey(flashAccounts.targetOracle), isSigner: false, isWritable: false },
    // 10: collateral_custody
    { pubkey: new PublicKey(flashAccounts.collateralCustody), isSigner: false, isWritable: true },
    // 11: collateral_oracle_account
    { pubkey: new PublicKey(flashAccounts.collateralOracle), isSigner: false, isWritable: false },
    // 12: collateral_custody_token_account
    { pubkey: new PublicKey(flashAccounts.collateralCustodyTokenAccount), isSigner: false, isWritable: true },
    // 13: token_program
    { pubkey: new PublicKey(flashAccounts.tokenProgram), isSigner: false, isWritable: false },
    // 14: event_authority
    { pubkey: new PublicKey(flashAccounts.eventAuthority), isSigner: false, isWritable: false },
    // 15: program (Flash Trade program)
    { pubkey: new PublicKey(flashAccounts.flashProgram), isSigner: false, isWritable: false },
    // 16: ix_sysvar
    { pubkey: new PublicKey(flashAccounts.ixSysvar), isSigner: false, isWritable: false },
    // 17: collateral_mint
    { pubkey: new PublicKey(flashAccounts.collateralMint), isSigner: false, isWritable: false },
  ];

  const tx = await program.methods
    .closeFlashPosition(
      oraclePrice ?? 0, // 0 = program fetches oracle price
      slippageBps,
      0, // delta — program computes
    )
    .accounts({
      treasury: treasuryPDA,
      strategyRecord: strategyPDA,
      authority: isKeypair(payer) ? payer.publicKey : (payer as WalletAdapter).publicKey!,
    })
    .remainingAccounts(remainingAccounts)
    .transaction();

  const signature = await sendTx(connection, tx, payer);
  return { signature, positionAddress };
}

export interface EmergencyResetCountersResult {
  signature: string;
  positionsReset: number;
}

/**
 * Emergency reset of Flash Trade position counters.
 *
 * Authority-gated. **Does NOT close actual Flash Trade positions on-chain.**
 * This only zeroes the `open_position_count` and `committed_sol_lamports` fields.
 *
 * Operators MUST call `closeFlashPosition` for each open position separately,
 * or rely on Flash Trade keeper liquidation, to actually unwind exposure.
 *
 * Use together with `freezeTreasury` for a full emergency halt.
 */
export async function emergencyResetPositionCounters(
  connection: Connection,
  payer: Keypair | WalletAdapter,
  authorityAddress: string | PublicKey,
  /** List of Flash Trade position addresses that were open. Used only for the event. */
  positionAddresses: string[],
): Promise<EmergencyResetCountersResult> {
  const authority = typeof authorityAddress === "string" ? new PublicKey(authorityAddress) : authorityAddress;
  const [treasuryPDA] = deriveTreasuryPDA(authority);
  const [strategyPDA] = deriveStrategyPDA(treasuryPDA, "SOL_2.69");

  const idl = loadPatchedIdl();
  const provider = new AnchorProvider(
    connection,
    isKeypair(payer) ? kpWallet(payer) : walletToAnchorWallet(payer),
    { commitment: "confirmed" },
  );
  const program = new Program(idl, provider);

  const positionPdas = positionAddresses.map((addr) => new PublicKey(addr));

  const tx = await program.methods
    .emergencyCloseAllPositions(positionPdas)
    .accounts({
      treasury: treasuryPDA,
      strategyRecord: strategyPDA,
      authority: isKeypair(payer) ? payer.publicKey : (payer as WalletAdapter).publicKey!,
    })
    .transaction();

  const signature = await sendTx(connection, tx, payer);
  return { signature, positionsReset: positionAddresses.length };
}
