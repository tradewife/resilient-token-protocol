// Type declarations for cross-directory imports resolved by tsx at runtime.
// The SDK and scripts directories are resolved via ../../../ from cli/src/commands/.

declare module "../../../sdk/index.ts" {
  import { Connection, Keypair, PublicKey } from "@solana/web3.js";
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
    isFrozen: boolean;
  }
  export function registerWithRTP(connection: Connection, payer: Keypair | any, opts: { mint: PublicKey; platform: string; name: string; symbol: string; holdersWallet?: PublicKey; projectDevWallet?: PublicKey; ecosystemWallet?: PublicKey; minRunwayBalance?: number }): Promise<{ mint: string; signature: string; treasuryPDA: string; vaultPDA: string; adopterPDA: string; explorerUrl: string }>;
  export function fetchTreasuryState(connection: Connection, mint: PublicKey): Promise<TreasuryState>;
  export function withdrawAndRedistribute(connection: Connection, payer: Keypair | any, mint: PublicKey): Promise<{ withdrawSig?: string; redistributeSig?: string }>;
  export function registerAdopterBeta(connection: Connection, payer: Keypair | any, mint: PublicKey, betaExpiresAt: number): Promise<any>;
  export function endBeta(connection: Connection, payer: Keypair | any, mint: PublicKey): Promise<any>;
  export function fetchAdopterState(connection: Connection, mint: PublicKey): Promise<any>;
  export function freezeTreasury(connection: Connection, payer: Keypair | any, mint: PublicKey): Promise<{ signature: string }>;
  export function unfreezeTreasury(connection: Connection, payer: Keypair | any, mint: PublicKey): Promise<{ signature: string }>;
  export function isTreasuryFrozen(connection: Connection, mint: PublicKey): Promise<boolean>;
  export function registerStrategy(connection: Connection, payer: Keypair | any, mint: string, strategyId: string, promotionSharpeX100: number): Promise<{ strategyPDA: string; signature: string }>;
  export const RTP_PROGRAM_ID: PublicKey;
}

declare module "../../../scripts/fee-crank.ts" {
  import { Connection, Keypair, PublicKey } from "@solana/web3.js";
  export interface SweepFeesOptions { dryRun?: boolean; jitterMaxMs?: number; feeThreshold?: number; }
  export interface SweepFeesResult { withdrawSig?: string; redistributeSig?: string; }
  export function exportSweepFees(connection: Connection, payer: Keypair, mint: PublicKey, opts?: SweepFeesOptions): Promise<SweepFeesResult>;
}

declare module "../../../scripts/promote-strategy.ts" {
  import { Connection, Keypair } from "@solana/web3.js";
  export interface PromoteStrategyOptions { dryRun?: boolean; resultsDir?: string; }
  export interface PromoteStrategyResult { strategyPDA?: string; signature?: string; strategyId?: string; }
  export function exportPromoteStrategy(connection: Connection, payer: Keypair, mint: string, opts?: PromoteStrategyOptions): Promise<PromoteStrategyResult | null>;
}

declare module "../../../scripts/emergency-freeze.ts" {
  import { Connection, Keypair, PublicKey } from "@solana/web3.js";
  export function exportFreezeTreasury(connection: Connection, payer: Keypair, mint: PublicKey): Promise<{ signature: string }>;
  export function exportUnfreezeTreasury(connection: Connection, payer: Keypair, mint: PublicKey): Promise<{ signature: string }>;
  export function exportFreezeStatus(connection: Connection, mint: PublicKey): Promise<{ frozen: boolean }>;
}

declare module "../../../scripts/derive_flash_accounts.ts" {
  import { PublicKey } from "@solana/web3.js";
  export interface DerivedMarketResult { symbol: string; side: string; pool: string; marketAddress: string; custodyAddress: string; oracleAddress: string; custodyTokenAccount: string; positionPda: string; openAccounts: number; closeAccounts: number; }
  export interface DerivedAccountsResult { network: string; programId: string; owner: string; perpetualsPda: string; transferAuthority: string; eventAuthority: string; markets: DerivedMarketResult[]; }
  export function exportDeriveAccounts(owner: PublicKey, network: "mainnet" | "devnet"): DerivedAccountsResult;
}

declare module "../../../scripts/compute_adopter_yield_share.ts" {
  export function computeAdopterYieldShares(input: any): any[];
}
