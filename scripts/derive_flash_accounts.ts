/**
 * Flash Trade Account Derivation Helper
 *
 * Pre-computes all Flash Trade PDA addresses and account pubkeys needed
 * for the open_flash_position / close_flash_position CPI instructions.
 * Runs offline (no RPC needed) using the same derivation as Flash Trade's SDK.
 *
 * Usage (run from a directory with @solana/web3.js installed):
 *   cd rtp/programs/rtp-treasury && npx tsx ../../../scripts/derive_flash_accounts.ts
 *   cd rtp/programs/rtp-treasury && npx tsx ../../../scripts/derive_flash_accounts.ts --owner <TREASURY_PDA>
 *
 *   # Or from the ts-test directory:
 *   cd /tmp/flash-cpi-poc/ts-test && npx tsx /path/to/scripts/derive_flash_accounts.ts
 *
 * Output: JSON with all account addresses for each supported market.
 */

import { PublicKey } from "@solana/web3.js";

// ─── Program IDs ────────────────────────────────────────────────────────

const FLASH_PROGRAM_ID = new PublicKey(
  "FLASH6Lo6h3iasJKWDs2F8TkW2UKf3s15C8PMGuVfgBn"
);
const FLASH_DEVNET_PROGRAM_ID = new PublicKey(
  "FTPP4jEWW1n8s2FEccwVfS9KCPjpndaswg7Nkkuz4ER4"
);

// ─── Well-Known Accounts (Crypto.1 Pool) ────────────────────────────────
// These are loaded from Flash Trade's on-chain PoolConfig and do not change.

interface MarketConfig {
  symbol: string;
  side: "Long" | "Short";
  marketAddress: string;
  custodyAddress: string;
  oracleAddress: string;
  custodyTokenAccount: string;
}

const CRYPTO1_POOL = "HfF7GCcEc76xubFCHLLXRdYcgRzwjEPdfKWqzRS8Ncog";
const PERPETUALS_PDA = "7DWCtB5Z8rPiyBMKUwqyC95R9tJpbhoQhLM9LbK3Z5QZ";

// SOL markets in Crypto.1 pool (from on-chain data)
const SOL_MARKETS: MarketConfig[] = [
  {
    symbol: "SOL",
    side: "Long",
    marketAddress: "3vHoXbUvGhEHFsLUmxyC6VWsbYDreb1zMn9TAp5ijN5K",
    custodyAddress: "BjzZ33nMnbXZ7rw3Uy9Uu1W7BDCzzugqkiZoamJHRKF7",
    oracleAddress: "DXqtMo8qRBfHcK11kBnSaCSXkWKk1huMf94R6sAxLHtf",
    custodyTokenAccount: "Hhed3wTHoVoPpnuBntGf236UfowMMAXfxqTLkMyJJENe",
  },
];

// ─── PDA Derivation ─────────────────────────────────────────────────────

function derivePDAs(owner: PublicKey, network: "mainnet" | "devnet") {
  const programId =
    network === "mainnet" ? FLASH_PROGRAM_ID : FLASH_DEVNET_PROGRAM_ID;

  // Global PDAs
  const [transferAuthority] = PublicKey.findProgramAddressSync(
    [Buffer.from("transfer_authority")],
    programId
  );
  const [eventAuthority] = PublicKey.findProgramAddressSync(
    [Buffer.from("__event_authority")],
    programId
  );

  const markets = SOL_MARKETS.map((m) => {
    const marketPk = new PublicKey(m.marketAddress);

    // Position PDA: ["position", owner, market] (3 seeds, verified M1)
    const [positionPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("position"), owner.toBuffer(), marketPk.toBuffer()],
      programId
    );

    return {
      symbol: m.symbol,
      side: m.side,
      pool: CRYPTO1_POOL,
      marketAddress: m.marketAddress,
      custodyAddress: m.custodyAddress,
      oracleAddress: m.oracleAddress,
      custodyTokenAccount: m.custodyTokenAccount,
      positionPda: positionPda.toBase58(),
      openAccounts: 19, // v15.2.0 IDL
      closeAccounts: 18, // v15.2.0 IDL
    };
  });

  return {
    network,
    programId: programId.toBase58(),
    owner: owner.toBase58(),
    perpetualsPda: PERPETUALS_PDA,
    transferAuthority: transferAuthority.toBase58(),
    eventAuthority: eventAuthority.toBase58(),
    markets,
  };
}

// ─── Main ───────────────────────────────────────────────────────────────

function main() {
  const args = process.argv.slice(2);
  const ownerIndex = args.indexOf("--owner");
  const ownerArg =
    ownerIndex >= 0 && args[ownerIndex + 1]
      ? args[ownerIndex + 1]
      : "FNQbK1Vw77aT7qM1EMSmeEPDGizSNhX4rkkYBKQNFotF"; // Default RTP treasury PDA
  const owner = new PublicKey(ownerArg);

  console.log("=== Flash Trade Account Derivation ===\n");
  console.log(`Owner (Treasury PDA): ${owner.toBase58()}\n`);

  const mainnet = derivePDAs(owner, "mainnet");

  console.log("Mainnet Accounts:");
  console.log(JSON.stringify(mainnet, null, 2));

  console.log("\n\nRust FlashTradeAccounts struct (for flash_trade_client.rs):");
  const m = mainnet.markets[0];
  console.log(`FlashTradeAccounts {
    program_id: "${mainnet.programId}".to_string(),
    composability_program_id: "FSWAPViR8ny5K96hezav8jynVubP2dJ2L7SbKzds2hwm".to_string(),
    perpetuals_pda: "${mainnet.perpetualsPda}".to_string(),
    transfer_authority: "${mainnet.transferAuthority}".to_string(),
    event_authority: "${mainnet.eventAuthority}".to_string(),
    pool_address: "${m.pool}".to_string(),
    target_custody: "${m.custodyAddress}".to_string(),
    target_oracle: "${m.oracleAddress}".to_string(),
    collateral_custody: "${m.custodyAddress}".to_string(),
    collateral_oracle: "${m.oracleAddress}".to_string(),
    collateral_custody_token_account: "${m.custodyTokenAccount}".to_string(),
    market_address: "${m.marketAddress}".to_string(),
    position_pda: "${m.positionPda}".to_string(),
}`);

  console.log("\n\nOpen Position Account Order (19 accounts, v15.2.0 IDL):");
  const accountNames = [
    "owner (signer, writable)",
    "fee_payer (signer, writable)",
    "funding_account (writable)",
    "transfer_authority",
    "perpetuals",
    "pool (writable)",
    "position (writable)",
    "market (writable)",
    "target_custody",
    "target_oracle",
    "collateral_custody (writable)",
    "collateral_oracle",
    "collateral_custody_token_account (writable)",
    "system_program",
    "funding_token_program",
    "event_authority",
    "program",
    "ix_sysvar",
    "funding_mint",
  ];
  accountNames.forEach((name, i) => {
    console.log(`  [${i.toString().padStart(2)}] ${name}`);
  });
}

main();
