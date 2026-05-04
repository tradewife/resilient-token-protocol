/**
 * Colosseum Demo Adopter — registers a real token project with RTP on devnet.
 *
 * Usage:
 *   npx tsx scripts/colosseum-adopter.ts
 *
 * This demonstrates that a real token project can adopt RTP with one call.
 * The adopter record is live on devnet and queryable on Solana Explorer.
 */
import {
  Connection,
  Keypair,
  PublicKey,
  Transaction,
  SystemProgram,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import {
  getProvider,
  Program,
  BN,
  AnchorProvider,
  Wallet,
  BorshCoder,
} from "@coral-xyz/anchor";
import { IDL } from "../sdk/idl";

const PROGRAM_ID = new PublicKey(
  "8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB"
);
const RPC = "https://api.devnet.solana.com";

// Demo adopter — "SolanaCat" token project adopting RTP
const ADOPTER_ID = "colosseum_demo_cat";

async function main() {
  console.log("=== Colosseum Demo Adopter ===");
  console.log(
    "Registering a token project with RTP on devnet...\n"
  );

  // Load fee-payer keypair
  const home = process.env.HOME || process.env.USERPROFILE || "";
  const keypairPath = `${home}/.config/solana/id.json`;
  let payer: Keypair;
  try {
    const fs = await import("fs");
    const keypairData = JSON.parse(fs.readFileSync(keypairPath, "utf-8"));
    payer = Keypair.fromSecretKey(new Uint8Array(keypairData));
  } catch {
    console.error(
      `Fee-payer keypair not found at ${keypairPath}. Run 'solana-keygen new' first.`
    );
    process.exit(1);
  }

  console.log(`Fee-payer: ${payer.publicKey.toBase58()}`);

  const connection = new Connection(RPC, "confirmed");

  // Check SOL balance
  const balance = await connection.getBalance(payer.publicKey);
  console.log(`SOL balance: ${(balance / LAMPORTS_PER_SOL).toFixed(4)} SOL`);

  if (balance < 0.05 * LAMPORTS_PER_SOL) {
    console.log("Low balance. Requesting airdrop...");
    const sig = await connection.requestAirdrop(
      payer.publicKey,
      0.5 * LAMPORTS_PER_SOL
    );
    await connection.confirmTransaction(sig, "confirmed");
    console.log("Airdrop confirmed.");
  }

  // Create provider
  const wallet: Wallet = {
    publicKey: payer.publicKey,
    signTransaction: async (tx) => {
      tx.partialSign(payer);
      return tx;
    },
    signAllTransactions: async (txs) => {
      return txs.map((tx) => {
        tx.partialSign(payer);
        return tx;
      });
    },
    payer,
  };

  const provider = new AnchorProvider(connection, wallet, {
    commitment: "confirmed",
  });
  const program = new Program(IDL, PROGRAM_ID, provider);

  // First, initialize a treasury if we need one
  // Use the payer as authority
  console.log(`\nAuthority: ${payer.publicKey.toBase58()}`);

  // Derive treasury PDA
  const [treasuryPDA] = PublicKey.findProgramAddressSync(
    [Buffer.from("treasury"), payer.publicKey.toBuffer()],
    PROGRAM_ID
  );
  console.log(`Treasury PDA: ${treasuryPDA.toBase58()}`);

  // Check if treasury exists
  let treasuryExists = false;
  try {
    await program.account.treasury.fetch(treasuryPDA);
    treasuryExists = true;
    console.log("Treasury exists.");
  } catch {
    console.log("Treasury not found. Initializing...");
    const holders = Keypair.generate().publicKey;
    const dev = Keypair.generate().publicKey;
    const ecosystem = Keypair.generate().publicKey;

    const initTx = await program.methods
      .initialize(holders, dev, ecosystem, new BN(0.1 * LAMPORTS_PER_SOL))
      .accounts({
        treasury: treasuryPDA,
        authority: payer.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([payer])
      .rpc();

    console.log(`Treasury initialized! TX: ${initTx}`);
    console.log(
      `Explorer: https://explorer.solana.com/tx/${initTx}?cluster=devnet`
    );
  }

  // Derive adopter PDA
  const [adopterPDA] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("adopter"),
      treasuryPDA.toBuffer(),
      Buffer.from(ADOPTER_ID),
    ],
    PROGRAM_ID
  );
  console.log(`\nAdopter PDA: ${adopterPDA.toBase58()}`);
  console.log(`Adopter ID: ${ADOPTER_ID}`);

  // Check if adopter already exists
  try {
    const adopter = await program.account.adopterRecord.fetch(adopterPDA);
    console.log("\nAdopter already registered!");
    console.log(`  Fees contributed: ${adopter.feesContributedLamports.toString()} lamports`);
    console.log(`  Deposits: ${adopter.depositCount.toString()}`);
    console.log(
      `  Adopted at: ${new Date(adopter.adoptedAt.toNumber() * 1000).toISOString()}`
    );
    return;
  } catch {
    // Adopter doesn't exist — register it
  }

  // Register adopter
  console.log("Registering adopter...");
  const registerTx = await program.methods
    .registerAdopter(ADOPTER_ID)
    .accounts({
      adopterRecord: adopterPDA,
      treasury: treasuryPDA,
      authority: payer.publicKey,
      systemProgram: SystemProgram.programId,
    })
    .signers([payer])
    .rpc();

  console.log(`\nAdopter registered! TX: ${registerTx}`);
  console.log(
    `Explorer: https://explorer.solana.com/tx/${registerTx}?cluster=devnet`
  );

  // Deposit some fees to simulate a real token project
  console.log("\nDepositing 0.01 SOL as simulated trading fees...");
  const depositTx = await program.methods
    .depositSol(new BN(0.01 * LAMPORTS_PER_SOL))
    .accounts({
      treasury: treasuryPDA,
      payer: payer.publicKey,
      systemProgram: SystemProgram.programId,
    })
    .signers([payer])
    .rpc();

  console.log(`Fees deposited! TX: ${depositTx}`);
  console.log(
    `Explorer: https://explorer.solana.com/tx/${depositTx}?cluster=devnet`
  );

  // Record the fee attribution
  console.log("\nRecording fee attribution...");
  const recordTx = await program.methods
    .recordFeeDeposit(new BN(0.01 * LAMPORTS_PER_SOL))
    .accounts({
      adopterRecord: adopterPDA,
      treasury: treasuryPDA,
      authority: payer.publicKey,
    })
    .signers([payer])
    .rpc();

  console.log(`Fee recorded! TX: ${recordTx}`);
  console.log(
    `Explorer: https://explorer.solana.com/tx/${recordTx}?cluster=devnet`
  );

  // Verify final state
  console.log("\n=== Verification ===");
  const treasury = await program.account.treasury.fetch(treasuryPDA);
  const adopter = await program.account.adopterRecord.fetch(adopterPDA);

  console.log(`Treasury SOL: ${treasury.totalFeesReceivedLamports.toString()} lamports`);
  console.log(`Adopter fees: ${adopter.feesContributedLamports.toString()} lamports`);
  console.log(`Phase: ${Object.keys(adopter)[0]}`);

  console.log(
    `\nView on Explorer: https://explorer.solana.com/address/${treasuryPDA.toBase58()}?cluster=devnet`
  );
  console.log(
    `View adopter: https://explorer.solana.com/address/${adopterPDA.toBase58()}?cluster=devnet`
  );

  console.log(
    "\nDone. A real token project (Colosseum Demo Cat) is now using RTP."
  );
}

main().catch((err) => {
  console.error("Error:", err.message || err);
  process.exit(1);
});

export {};
