// @resilient-protocol/sdk
// The ONLY function a launchpad needs to call.

import {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
  sendAndConfirmTransaction,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";

import {
  TOKEN_2022_PROGRAM_ID,
  createInitializeMintInstruction,
  createInitializeTransferFeeConfigInstruction,
  getMintLen,
  ExtensionType,
  mintTo,
  createAssociatedTokenAccount,
  getAssociatedTokenAddress,
  TYPE_SIZE,
  LENGTH_SIZE,
} from "@solana/spl-token";

// ── Constants ────────────────────────────────────────────────

/** The RTP treasury vault — hardcoded. Launchpads cannot redirect fees. */
export const RTP_TREASURY_VAULT = "FNQbK1Vw77aT7qM1EMSmeEPDGizSNhX4rkkYBKQNFotF";

/** Devnet RPC endpoint. */
export const RTP_DEVNET_RPC = "https://api.devnet.solana.com";

/** Mainnet RPC endpoint. */
export const RTP_MAINNET_RPC = "https://api.mainnet-beta.solana.com";

// ── Types ────────────────────────────────────────────────────

export interface RTPTokenConfig {
  name: string;
  symbol: string;
  supply: number;       // in tokens (not lamports)
  feeBps: number;       // transfer fee in basis points e.g. 200 = 2%
  decimals?: number;    // default 6
}

export interface RTPTokenResult {
  mint: string;         // base58 mint address
  signature: string;    // tx signature
  explorerUrl: string;  // devnet explorer link
  treasuryVault: string; // the RTP treasury PDA this token feeds
}

/** Minimal wallet adapter interface — compatible with @solana/wallet-adapter-react */
export interface WalletAdapter {
  publicKey: PublicKey | null;
  signTransaction<T extends Transaction>(tx: T): Promise<T>;
}

// ── Implementation ───────────────────────────────────────────

/**
 * Create a Token-2022 mint with transfer fee destination set to the
 * RTP treasury vault. This is the single integration point for launchpads.
 *
 * The treasury vault address is hardcoded — launchpads cannot redirect fees.
 * This is a constitutional invariant of the protocol.
 */
export async function createRTPToken(
  connection: Connection,
  payer: Keypair | WalletAdapter,
  config: RTPTokenConfig,
): Promise<RTPTokenResult> {
  const decimals = config.decimals ?? 6;
  const treasuryVault = new PublicKey(RTP_TREASURY_VAULT);

  // Derive the fee in basis-point format (max 500 = 5%)
  const feeBps = Math.min(config.feeBps, 500);

  // Calculate mint account space (TransferFeeConfig extension + Mint)
  const mintLen = getMintLen([ExtensionType.TransferFeeConfig]);

  // Mint keypair — launchpad controls the mint authority
  const mintKeypair = Keypair.generate();
  const mintPubkey = mintKeypair.publicKey;

  // Minimum balance for rent-exempt mint account
  const lamports = await connection.getMinimumBalanceForRentExemption(mintLen);

  // Build transaction
  const transaction = new Transaction();

  // 1. Create the mint account
  transaction.add(
    SystemProgram.createAccount({
      fromPubkey: payer instanceof Keypair ? payer.publicKey : payer.publicKey!,
      newAccountPubkey: mintPubkey,
      space: mintLen,
      lamports,
      programId: TOKEN_2022_PROGRAM_ID,
    }),
  );

  // 2. Initialize TransferFeeConfig — fees route to RTP treasury vault
  transaction.add(
    createInitializeTransferFeeConfigInstruction(
      mintPubkey,                              // mint
      payer instanceof Keypair ? payer.publicKey : payer.publicKey!, // fee authority (launchpad)
      treasuryVault,                           // fee destination ← hardcoded to RTP
      feeBps,                                  // transfer fee basis points
      Math.max(feeBps, 500),                   // maximum fee (at least equal to fee)
      TOKEN_2022_PROGRAM_ID,
    ),
  );

  // 3. Initialize the mint
  transaction.add(
    createInitializeMintInstruction(
      mintPubkey,
      decimals,
      payer instanceof Keypair ? payer.publicKey : payer.publicKey!, // mint authority (launchpad)
      payer instanceof Keypair ? payer.publicKey : payer.publicKey!, // freeze authority (launchpad)
      TOKEN_2022_PROGRAM_ID,
    ),
  );

  // Get recent blockhash
  const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
  transaction.recentBlockhash = blockhash;
  transaction.lastValidBlockHeight = lastValidBlockHeight;
  transaction.feePayer = payer instanceof Keypair ? payer.publicKey : payer.publicKey!;

  let signature: string;

  if (payer instanceof Keypair) {
    // Keypair path — sign with both payer and mint keypair
    transaction.partialSign(payer, mintKeypair);
    signature = await sendAndConfirmTransaction(connection, transaction, [payer, mintKeypair]);
  } else {
    // Wallet adapter path — sign with wallet, then mint keypair is a partial signer
    // NOTE: In production, the launchpad backend holds the mint keypair.
    // For the SDK demo flow, we sign the mint keypair first, then the wallet.
    transaction.partialSign(mintKeypair);
    const signed = await payer.signTransaction(transaction);
    signature = await sendAndConfirmTransaction(connection, signed, []);
  }

  // 4. Mint initial supply to the launchpad's token account
  const supplyLamports = BigInt(config.supply) * BigInt(10 ** decimals);
  const payerPubkey = payer instanceof Keypair ? payer.publicKey : payer.publicKey!;

  if (payer instanceof Keypair) {
    // Create ATA and mint supply
    const ata = await createAssociatedTokenAccount(
      connection,
      payer,
      mintPubkey,
      payer.publicKey,
      payer,
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
    treasuryVault: RTP_TREASURY_VAULT,
  };
}
