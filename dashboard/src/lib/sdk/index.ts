// @resilient-protocol/sdk
// The launchpad integration SDK for the Resilient Token Protocol.
// Core functions: registerWithRTP, fetchTreasuryState, withdrawAndRedistribute.

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

import {
  TOKEN_2022_PROGRAM_ID,
  createInitializeMintInstruction,
  createInitializeTransferFeeConfigInstruction,
  getMintLen,
  ExtensionType,
  createMintToInstruction,
  createAssociatedTokenAccountInstruction,
  getAssociatedTokenAddressSync,
  getAccount,
  getMint,
  Account,
} from "@solana/spl-token";

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
const SEED_VAULT = Buffer.from("vault");
const SEED_ADOPTER = Buffer.from("adopter");

function deriveTreasuryPDA(mint: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEED_TREASURY, mint.toBuffer()],
    RTP_PROGRAM_ID,
  );
}

function deriveVaultPDA(mint: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEED_TREASURY, mint.toBuffer(), SEED_VAULT],
    RTP_PROGRAM_ID,
  );
}

function deriveAdopterPDA(mint: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [SEED_ADOPTER, mint.toBuffer()],
    RTP_PROGRAM_ID,
  );
}

// IDL (bundled inline — no runtime file dependency)

import { RAW_IDL } from "./idl";

function loadPatchedIdl(): Idl {
  const idl = JSON.parse(JSON.stringify(RAW_IDL));
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
  /** The token mint to register with RTP */
  mint: PublicKey;
  /** The platform the token was launched on */
  platform: "pumpfun" | "bags" | "raydium";
  /** Token display name */
  name: string;
  /** Token ticker symbol */
  symbol: string;
  /** Wallet receiving 70% holder distributions (default: payer) */
  holdersWallet?: PublicKey;
  /** Wallet receiving 20% dev distributions (default: payer) */
  projectDevWallet?: PublicKey;
  /** Wallet receiving 10% ecosystem distributions (default: payer) */
  ecosystemWallet?: PublicKey;
  /** Minimum runway balance in token lamports (default: 10_000_000) */
  minRunwayBalance?: number;
}

export interface RTPRegistrationResult {
  /** The mint address (base58) */
  mint: string;
  /** Transaction signature of the registration */
  signature: string;
  /** Solana Explorer link */
  explorerUrl: string;
  /** Per-mint treasury state account */
  treasuryPDA: string;
  /** Token account receiving fees */
  vaultPDA: string;
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
  mint: string;
  phase: "Sustenance" | "Ecosystem" | "Humanity";
  vaultBalance: number;
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
  tx.feePayer = payer instanceof Keypair ? payer.publicKey : payer.publicKey!;

  if (payer instanceof Keypair) {
    return sendAndConfirmTransaction(connection, tx, [payer, ...extraSigners]);
  } else {
    if (extraSigners.length > 0) tx.partialSign(...extraSigners);
    const signed = await payer.signTransaction(tx);
    return sendSignedTx(connection, signed);
  }
}

// Implementation

/**
 * Register an existing token mint with the RTP protocol.
 * Creates the per-mint treasury PDA, vault PDA, and adopter record on-chain.
 *
 * This is the single integration point for launchpads — call it after
 * your platform has created the token mint.
 */
export async function registerWithRTP(
  connection: Connection,
  payer: Keypair | WalletAdapter,
  config: RTPRegistrationConfig,
): Promise<RTPRegistrationResult> {
  const payerPubkey = payer instanceof Keypair ? payer.publicKey : payer.publicKey!;
  const mintPubkey = config.mint;

  // Derive per-mint PDAs
  const [treasuryPDA] = deriveTreasuryPDA(mintPubkey);
  const [vaultPDA] = deriveVaultPDA(mintPubkey);
  const [adopterPDA] = deriveAdopterPDA(mintPubkey);

  const holdersWallet = config.holdersWallet ?? payerPubkey;
  const projectDevWallet = config.projectDevWallet ?? payerPubkey;
  const ecosystemWallet = config.ecosystemWallet ?? payerPubkey;
  const minRunwayBalance = config.minRunwayBalance ?? 10_000_000;

  // Read mint decimals from on-chain
  let decimals = 6;
  try {
    const mintInfo = await getMint(connection, mintPubkey, "confirmed", TOKEN_2022_PROGRAM_ID);
    decimals = mintInfo.decimals;
  } catch {
    // Token-2022 fetch may fail for standard SPL tokens — try standard program
    try {
      const mintInfo = await getMint(connection, mintPubkey, "confirmed");
      decimals = mintInfo.decimals;
    } catch {
      // Use default 6 decimals
    }
  }

  // Detect token program for this mint
  const mintAccount = await connection.getAccountInfo(mintPubkey);
  const tokenProgram = mintAccount?.owner || TOKEN_2022_PROGRAM_ID;

  // Build the initialize instruction via Anchor
  const idl = loadPatchedIdl();
  const provider = new AnchorProvider(
    connection,
    payer instanceof Keypair ? kpWallet(payer) : walletToAnchorWallet(payer),
    { commitment: "confirmed" },
  );
  const program = new Program(idl, provider);

  // Step 1: Initialize treasury
  const initTx = await program.methods
    .initialize(new BN(minRunwayBalance))
    .accounts({
      mint: mintPubkey,
      treasury: treasuryPDA,
      treasuryVault: vaultPDA,
      holdersWallet,
      projectDevWallet,
      ecosystemWallet,
      authority: payerPubkey,
      tokenProgram,
      systemProgram: SystemProgram.programId,
    })
    .transaction();

  const signature = await sendTx(connection, initTx, payer);

  // Step 2: Register adopter
  try {
    const adopterTx = await program.methods
      .registerAdopter(mintPubkey)
      .accounts({
        adopterRecord: adopterPDA,
        treasury: treasuryPDA,
        authority: payerPubkey,
        systemProgram: SystemProgram.programId,
      })
      .transaction();

    await sendTx(connection, adopterTx, payer);
  } catch (e: unknown) {
    // Adopter registration is best-effort — treasury is the critical path
    console.warn("[RTP SDK] Adopter registration skipped:", e instanceof Error ? e.message : String(e));
  }

  const cluster = connection.rpcEndpoint.includes("devnet") ? "devnet" : "mainnet-beta";

  return {
    mint: mintPubkey.toBase58(),
    signature,
    explorerUrl: `https://explorer.solana.com/tx/${signature}?cluster=${cluster}`,
    treasuryPDA: treasuryPDA.toBase58(),
    vaultPDA: vaultPDA.toBase58(),
    adopterPDA: adopterPDA.toBase58(),
  };
}

/**
 * Fetch the on-chain treasury state for a given mint.
 * Read-only — no transactions, no signing required.
 * Returns zeros if the treasury account doesn't exist yet.
 */
export async function fetchTreasuryState(
  connection: Connection,
  mintAddress: string | PublicKey,
): Promise<TreasuryState> {
  const mint = typeof mintAddress === "string" ? new PublicKey(mintAddress) : mintAddress;
  const [treasuryPDA] = deriveTreasuryPDA(mint);
  const [vaultPDA] = deriveVaultPDA(mint);

  const idl = loadPatchedIdl();
  const coder = new BorshCoder(idl);

  // Fetch treasury account
  const accountInfo = await connection.getAccountInfo(treasuryPDA);
  if (!accountInfo) {
    return {
      mint: mint.toBase58(),
      phase: "Sustenance",
      vaultBalance: 0,
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

  // Fetch vault token balance
  let vaultBalance = 0;
  try {
    const vaultAccount = await getAccount(connection, vaultPDA, "confirmed", TOKEN_2022_PROGRAM_ID);
    vaultBalance = Number(vaultAccount.amount);
  } catch {
    // Vault token account doesn't exist yet — expected before first fee deposit
  }

  // Read actual decimals from the mint account (not hardcoded)
  let decimals = 6;
  try {
    const mintInfo = await getMint(connection, mint, "confirmed", TOKEN_2022_PROGRAM_ID);
    decimals = mintInfo.decimals;
  } catch {
    // Mint not reachable (network error, wrong cluster) — use default 6 decimals
  }

  return {
    mint: mint.toBase58(),
    phase,
    vaultBalance: vaultBalance / 10 ** decimals,
    totalFeesWithdrawn: Number(treasury.totalFeesWithdrawn) / 10 ** decimals,
    totalDistributedHolders: Number(treasury.totalDistributedHolders) / 10 ** decimals,
    totalDistributedDev: Number(treasury.totalDistributedDev) / 10 ** decimals,
    totalDistributedEcosystem: Number(treasury.totalDistributedEcosystem) / 10 ** decimals,
    totalHydration: Number(treasury.totalHydration) / 10 ** decimals,
    totalFeesReceived: Number(treasury.totalFeesReceivedLamports) / 10 ** decimals,
    minRunwayBalance: Number(treasury.minRunwayBalance) / 10 ** decimals,
    isFrozen: Boolean(treasury.frozen),
  };
}

/**
 * Permissionless crank: withdraw fees from the mint into the treasury vault,
 * then redistribute (70/20/10 split) if above the runway threshold.
 *
 * Anyone can call this — the launchpad, a keeper bot, or any user.
 */
export async function withdrawAndRedistribute(
  connection: Connection,
  payer: Keypair | WalletAdapter,
  mintAddress: string | PublicKey,
): Promise<{ withdrawSig: string; redistributeSig?: string }> {
  const mint = typeof mintAddress === "string" ? new PublicKey(mintAddress) : mintAddress;
  const [treasuryPDA] = deriveTreasuryPDA(mint);
  const [vaultPDA] = deriveVaultPDA(mint);

  const idl = loadPatchedIdl();
  const payerPubkey = payer instanceof Keypair ? payer.publicKey : payer.publicKey!;
  const provider = new AnchorProvider(
    connection,
    payer instanceof Keypair ? kpWallet(payer) : walletToAnchorWallet(payer),
    { commitment: "confirmed" },
  );
  const program = new Program(idl, provider);

  // Step 1: Withdraw fees from mint into treasury vault
  const withdrawTx = await program.methods
    .withdrawFees()
    .accounts({
      mint,
      treasury: treasuryPDA,
      treasuryVault: vaultPDA,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
    })
    .transaction();

  const withdrawSig = await sendTx(connection, withdrawTx, payer);

  // Step 2: Try to redistribute if above threshold
  try {
    // Fetch treasury state to get wallet addresses for ATAs
    const coder = new BorshCoder(idl);
    const accountInfo = await connection.getAccountInfo(treasuryPDA);
    if (!accountInfo) {
      return { withdrawSig };
    }

    const treasury = coder.accounts.decode("Treasury", accountInfo.data);

    const holdersATA = getAssociatedTokenAddressSync(
      mint, treasury.holdersWallet, false, TOKEN_2022_PROGRAM_ID,
    );
    const devATA = getAssociatedTokenAddressSync(
      mint, treasury.projectDevWallet, false, TOKEN_2022_PROGRAM_ID,
    );
    const ecosystemATA = getAssociatedTokenAddressSync(
      mint, treasury.ecosystemWallet, false, TOKEN_2022_PROGRAM_ID,
    );

    const redistributeTx = await program.methods
      .checkRedistribute()
      .accounts({
        mint,
        treasury: treasuryPDA,
        treasuryVault: vaultPDA,
        holdersRecipient: holdersATA,
        devRecipient: devATA,
        ecosystemRecipient: ecosystemATA,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
      })
      .transaction();

    const redistributeSig = await sendTx(connection, redistributeTx, payer);

    return { withdrawSig, redistributeSig };
  } catch (err: unknown) {
    // BelowThreshold (error code 6000) is expected — vault doesn't have enough yet
    const anchorErr = err as {
      error?: { errorCode?: { code?: string; number?: number } };
      message?: string;
    };
    const isBelowThreshold =
      anchorErr.error?.errorCode?.code === "BelowThreshold" ||
      anchorErr.message?.includes("BelowThreshold") ||
      anchorErr.error?.errorCode?.number === 6000;

    if (isBelowThreshold) {
      return { withdrawSig };
    }
    throw err;
  }
}

// ---------------------------------------------------------------------------
// Beta Adopter Functions
// ---------------------------------------------------------------------------

export interface AdopterState {
  tokenMint: string;
  feesContributed: number;
  adoptedAt: number;
  lastDepositAt: number;
  depositCount: number;
  betaExpiresAt: number;  // 0 = permanent, >0 = unix timestamp
  betaEnded: boolean;
  isBeta: boolean;
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
export async function registerAdopterBeta(
  connection: Connection,
  payer: Keypair | WalletAdapter,
  mintAddress: string | PublicKey,
  betaExpiresAt: number,
): Promise<{ signature: string; adopterPDA: string }> {
  const mint = typeof mintAddress === "string" ? new PublicKey(mintAddress) : mintAddress;
  const [treasuryPDA] = deriveTreasuryPDA(mint);
  const [adopterPDA] = deriveAdopterPDA(mint);
  const payerPubkey = payer instanceof Keypair ? payer.publicKey : payer.publicKey!;

  const idl = loadPatchedIdl();
  const provider = new AnchorProvider(
    connection,
    payer instanceof Keypair ? kpWallet(payer) : walletToAnchorWallet(payer),
    { commitment: "confirmed" },
  );
  const program = new Program(idl, provider);

  const tx = await program.methods
    .registerAdopterBeta(mint, new BN(betaExpiresAt))
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
 * @param mintAddress - The token mint whose beta to end
 */
export async function endBeta(
  connection: Connection,
  payer: Keypair | WalletAdapter,
  mintAddress: string | PublicKey,
): Promise<{ signature: string }> {
  const mint = typeof mintAddress === "string" ? new PublicKey(mintAddress) : mintAddress;
  const [treasuryPDA] = deriveTreasuryPDA(mint);
  const [adopterPDA] = deriveAdopterPDA(mint);

  const idl = loadPatchedIdl();
  const provider = new AnchorProvider(
    connection,
    payer instanceof Keypair ? kpWallet(payer) : walletToAnchorWallet(payer),
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
  mintAddress: string | PublicKey,
): Promise<AdopterState> {
  const mint = typeof mintAddress === "string" ? new PublicKey(mintAddress) : mintAddress;
  const [adopterPDA] = deriveAdopterPDA(mint);

  const idl = loadPatchedIdl();
  const coder = new BorshCoder(idl);

  const accountInfo = await connection.getAccountInfo(adopterPDA);
  if (!accountInfo) {
    return {
      tokenMint: mint.toBase58(),
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
    tokenMint: adopter.tokenMint.toBase58(),
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
  mintAddress: string | PublicKey,
): Promise<{ signature: string }> {
  const mint = typeof mintAddress === "string" ? new PublicKey(mintAddress) : mintAddress;
  const [treasuryPDA] = deriveTreasuryPDA(mint);
  const payerPubkey = payer instanceof Keypair ? payer.publicKey : payer.publicKey!;

  const idl = loadPatchedIdl();
  const provider = new AnchorProvider(
    connection,
    payer instanceof Keypair ? kpWallet(payer) : walletToAnchorWallet(payer),
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
  mintAddress: string | PublicKey,
): Promise<{ signature: string }> {
  const mint = typeof mintAddress === "string" ? new PublicKey(mintAddress) : mintAddress;
  const [treasuryPDA] = deriveTreasuryPDA(mint);
  const payerPubkey = payer instanceof Keypair ? payer.publicKey : payer.publicKey!;

  const idl = loadPatchedIdl();
  const provider = new AnchorProvider(
    connection,
    payer instanceof Keypair ? kpWallet(payer) : walletToAnchorWallet(payer),
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
  mintAddress: string | PublicKey,
): Promise<boolean> {
  const state = await fetchTreasuryState(connection, mintAddress);
  return state.isFrozen;
}
