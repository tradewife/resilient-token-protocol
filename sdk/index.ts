// @resilient-protocol/sdk
// The launchpad integration SDK for the Resilient Token Protocol.
// Three functions: createRTPToken, fetchTreasuryState, withdrawAndRedistribute.

import * as anchor from "@coral-xyz/anchor";
import { BN } from "@coral-xyz/anchor";

import {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";

import {
  TOKEN_2022_PROGRAM_ID,
  createInitializeMintInstruction,
  createInitializeTransferFeeConfigInstruction,
  getMintLen,
  ExtensionType,
  mintTo,
  createAssociatedTokenAccount,
  getAssociatedTokenAddressSync,
  getAccount,
} from "@solana/spl-token";

// ── Constants ────────────────────────────────────────────────

/** The RTP treasury Anchor program (deployed on devnet). */
export const RTP_PROGRAM_ID = new PublicKey(
  "8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB",
);

/** Devnet RPC endpoint. */
export const RTP_DEVNET_RPC = "https://api.devnet.solana.com";

/** Mainnet RPC endpoint. */
export const RTP_MAINNET_RPC = "https://api.mainnet-beta.solana.com";

// ── PDA Seeds ────────────────────────────────────────────────

const SEED_TREASURY = Buffer.from("treasury");
const SEED_VAULT = Buffer.from("vault");

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

// ── IDL Loader ───────────────────────────────────────────────

function loadPatchedIdl(): any {
  const rawIdl = require("../rtp/programs/rtp-treasury/target/idl/rtp_treasury.json");
  const idl = JSON.parse(JSON.stringify(rawIdl));
  idl.address = RTP_PROGRAM_ID.toBase58();
  // Anchor 0.31 workaround: accounts array entries are missing 'type'
  const typeMap: Record<string, any> = {};
  for (const t of idl.types || []) typeMap[t.name] = t;
  for (const acc of idl.accounts || []) {
    if (!acc.type && typeMap[acc.name]) acc.type = typeMap[acc.name].type;
  }
  return idl;
}

// ── Types ────────────────────────────────────────────────────

export interface RTPTokenConfig {
  name: string;
  symbol: string;
  supply: number;              // in tokens (not lamports)
  feeBps: number;              // transfer fee in basis points, e.g. 200 = 2%
  decimals?: number;           // default 6
  holdersWallet?: PublicKey;   // default: payer
  projectDevWallet?: PublicKey; // default: payer
  ecosystemWallet?: PublicKey; // default: payer
  minRunwayBalance?: number;   // default: 10_000_000 (10 tokens, 6 dec)
}

export interface RTPTokenResult {
  mint: string;           // base58 mint address
  signature: string;      // mint creation tx signature
  explorerUrl: string;    // devnet explorer link
  treasuryPDA: string;    // the treasury state account address
  vaultPDA: string;       // the treasury vault token account address
}

/** Minimal wallet adapter interface — compatible with @solana/wallet-adapter-react */
export interface WalletAdapter {
  publicKey: PublicKey | null;
  signTransaction<T extends Transaction>(tx: T): Promise<T>;
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
  minRunwayBalance: number;
}

// ── Implementation ───────────────────────────────────────────

/**
 * Create a Token-2022 mint with transfer fees routing to a per-mint
 * treasury vault PDA, and initialize the RTP treasury program.
 *
 * This is the single integration point for launchpads.
 */
export async function createRTPToken(
  connection: Connection,
  payer: Keypair | WalletAdapter,
  config: RTPTokenConfig,
): Promise<RTPTokenResult> {
  const decimals = config.decimals ?? 6;
  const payerPubkey = payer instanceof Keypair ? payer.publicKey : payer.publicKey!;

  // Mint keypair — launchpad controls the mint authority
  const mintKeypair = Keypair.generate();
  const mintPubkey = mintKeypair.publicKey;

  // Derive per-mint PDAs
  const [treasuryPDA] = deriveTreasuryPDA(mintPubkey);
  const [vaultPDA] = deriveVaultPDA(mintPubkey);

  // Cap fee at 500 bps (5%)
  const feeBps = Math.min(config.feeBps, 500);

  // Calculate mint account space
  const mintLen = getMintLen([ExtensionType.TransferFeeConfig]);
  const lamports = await connection.getMinimumBalanceForRentExemption(mintLen);

  // ── Transaction 1: Create Token-2022 mint with TransferFeeConfig ──

  const transaction = new Transaction();

  transaction.add(
    SystemProgram.createAccount({
      fromPubkey: payerPubkey,
      newAccountPubkey: mintPubkey,
      space: mintLen,
      lamports,
      programId: TOKEN_2022_PROGRAM_ID,
    }),
  );

  // Fee destination = treasury vault PDA for this specific mint
  transaction.add(
    createInitializeTransferFeeConfigInstruction(
      mintPubkey,
      payerPubkey,                  // fee config authority
      treasuryPDA,                  // withdraw_withheld_authority = Treasury PDA
      feeBps,
      BigInt(Math.max(feeBps, 500)),
      TOKEN_2022_PROGRAM_ID,
    ),
  );

  transaction.add(
    createInitializeMintInstruction(
      mintPubkey,
      decimals,
      payerPubkey,   // mint authority
      payerPubkey,   // freeze authority
      TOKEN_2022_PROGRAM_ID,
    ),
  );

  const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
  transaction.recentBlockhash = blockhash;
  transaction.lastValidBlockHeight = lastValidBlockHeight;
  transaction.feePayer = payerPubkey;

  let signature: string;

  if (payer instanceof Keypair) {
    signature = await sendAndConfirmTransaction(connection, transaction, [payer, mintKeypair]);
  } else {
    transaction.partialSign(mintKeypair);
    const signed = await payer.signTransaction(transaction);
    signature = await sendAndConfirmTransaction(connection, signed, []);
  }

  // ── Keypair path: initialize treasury + mint supply ──

  if (payer instanceof Keypair) {
    // Initialize the RTP treasury program for this mint
    const idl = loadPatchedIdl();
    const wallet = new anchor.Wallet(payer);
    const provider = new anchor.AnchorProvider(connection, wallet, { commitment: "confirmed" });
    const program = new anchor.Program(idl, provider);

    const holdersWallet = config.holdersWallet ?? payerPubkey;
    const projectDevWallet = config.projectDevWallet ?? payerPubkey;
    const ecosystemWallet = config.ecosystemWallet ?? payerPubkey;
    const minRunwayBalance = config.minRunwayBalance ?? 10_000_000;

    await program.methods
      .initialize(new BN(minRunwayBalance))
      .accounts({
        mint: mintPubkey,
        treasury: treasuryPDA,
        treasuryVault: vaultPDA,
        holdersWallet,
        projectDevWallet,
        ecosystemWallet,
        authority: payerPubkey,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    // Create ATA and mint initial supply
    const supplyLamports = BigInt(config.supply) * BigInt(10 ** decimals);

    const ata = await createAssociatedTokenAccount(
      connection,
      payer,
      mintPubkey,
      payer.publicKey,
      undefined,
      TOKEN_2022_PROGRAM_ID,
    );

    await mintTo(
      connection,
      payer,
      mintPubkey,
      ata,
      payer,
      supplyLamports,
      [],
      { commitment: "confirmed" },
      TOKEN_2022_PROGRAM_ID,
    );
  }

  const cluster = connection.rpcEndpoint.includes("devnet") ? "devnet" : "mainnet-beta";

  return {
    mint: mintPubkey.toBase58(),
    signature,
    explorerUrl: `https://explorer.solana.com/tx/${signature}?cluster=${cluster}`,
    treasuryPDA: treasuryPDA.toBase58(),
    vaultPDA: vaultPDA.toBase58(),
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
  const coder = new anchor.BorshCoder(idl);

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
      minRunwayBalance: 0,
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
    // Vault doesn't exist yet
  }

  const decimals = 6;

  return {
    mint: mint.toBase58(),
    phase,
    vaultBalance: vaultBalance / 10 ** decimals,
    totalFeesWithdrawn: Number(treasury.totalFeesWithdrawn) / 10 ** decimals,
    totalDistributedHolders: Number(treasury.totalDistributedHolders) / 10 ** decimals,
    totalDistributedDev: Number(treasury.totalDistributedDev) / 10 ** decimals,
    totalDistributedEcosystem: Number(treasury.totalDistributedEcosystem) / 10 ** decimals,
    totalHydration: Number(treasury.totalHydration) / 10 ** decimals,
    minRunwayBalance: Number(treasury.minRunwayBalance) / 10 ** decimals,
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
  payer: Keypair,
  mintAddress: string | PublicKey,
): Promise<{ withdrawSig: string; redistributeSig?: string }> {
  const mint = typeof mintAddress === "string" ? new PublicKey(mintAddress) : mintAddress;
  const [treasuryPDA] = deriveTreasuryPDA(mint);
  const [vaultPDA] = deriveVaultPDA(mint);

  const idl = loadPatchedIdl();
  const wallet = new anchor.Wallet(payer);
  const provider = new anchor.AnchorProvider(connection, wallet, { commitment: "confirmed" });
  const program = new anchor.Program(idl, provider);

  // Step 1: Withdraw fees from mint into treasury vault
  const withdrawSig = await program.methods
    .withdrawFees()
    .accounts({
      mint,
      treasury: treasuryPDA,
      treasuryVault: vaultPDA,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
    })
    .rpc();

  // Step 2: Try to redistribute if above threshold
  try {
    // Fetch treasury state to get wallet addresses for ATAs
    const coder = new anchor.BorshCoder(idl);
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

    const redistributeSig = await program.methods
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
      .rpc();

    return { withdrawSig, redistributeSig };
  } catch (err: any) {
    // BelowThreshold (error code 6000) is expected — vault doesn't have enough yet
    const isBelowThreshold =
      err?.error?.errorCode?.code === "BelowThreshold" ||
      err?.message?.includes("BelowThreshold") ||
      err?.error?.errorCode?.number === 6000;

    if (isBelowThreshold) {
      return { withdrawSig };
    }
    throw err;
  }
}
