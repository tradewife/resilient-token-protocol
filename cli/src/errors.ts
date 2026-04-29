// RTP CLI — Error types and formatting.

import chalk from "chalk";

export class RtpError extends Error {
  constructor(
    message: string,
    public readonly hint?: string,
    public readonly code: number = 1,
  ) {
    super(message);
    this.name = "RtpError";
  }
}

export function printError(err: unknown): void {
  if (err instanceof RtpError) {
    console.error(`\n${chalk.red("[rtp] ERROR:")} ${err.message}`);
    if (err.hint) console.error(`\n  Hint: ${err.hint}`);
  } else if (err instanceof Error) {
    console.error(`\n${chalk.red("[rtp] ERROR:")} ${err.message}`);
  } else {
    console.error(`\n${chalk.red("[rtp] ERROR:")} ${String(err)}`);
  }
  console.error();
}

// Specific error constructors

export function keypairError(path: string): RtpError {
  return new RtpError(
    `Keypair file not found: ${path}`,
    `Set --authority flag, KEYPAIR_PATH env var, or run 'rtp init'.`,
    2,
  );
}

export function authorityMismatchError(found: string, expected: string): RtpError {
  return new RtpError(
    `Authority ${found} does not match treasury.authority ${expected}`,
    `This operation requires the treasury authority keypair. Check SOULCONTRACT.md trust model.`,
    4,
  );
}

export function insufficientSolError(balance: number, minimum: number): RtpError {
  return new RtpError(
    `Fee-payer balance (${(balance / 1e9).toFixed(4)} SOL) below minimum (${(minimum / 1e9).toFixed(4)} SOL)`,
    `Fund wallet or run: solana airdrop 0.05 <pubkey> --url devnet`,
    3,
  );
}

export function treasuryFrozenError(mint: string): RtpError {
  return new RtpError(
    `Treasury is FROZEN — all operations halted`,
    `Run: rtp status --mint ${mint} to check state. Use 'rtp unfreeze' to resume.`,
    4,
  );
}

export function rpcUnreachableError(url: string): RtpError {
  return new RtpError(
    `Failed to connect to ${url}`,
    `Check cluster config (rtp init) or set SOLANA_RPC_URL env var.`,
    3,
  );
}

export function staleOracleError(): RtpError {
  return new RtpError(
    `StaleOraclePrice — Pyth oracles are mainnet-only`,
    `Flash Trade CPI requires mainnet. Devnet supports account derivation and constraint tests only. See CLAUDE.md Devnet Limitations.`,
    4,
  );
}

export function noNightResultsError(dir: string): RtpError {
  return new RtpError(
    `No night shift results found in ${dir}`,
    `Run the night shift first: python -m research.orchestration.night_shift --skip-fetch`,
    1,
  );
}

export function belowThresholdError(): RtpError {
  return new RtpError(
    `Vault balance below runway floor — redistribution skipped`,
    `This is expected when the vault has no excess above the runway floor.`,
    0, // Not really an error
  );
}

export function programNotDeployedError(programId: string, cluster: string): RtpError {
  return new RtpError(
    `Program ${programId} not found on ${cluster}`,
    `Deploy first: rtp deploy program --cluster ${cluster}`,
    4,
  );
}

export function missingYesFlagError(operation: string): RtpError {
  return new RtpError(
    `--yes required for ${operation}. Review the output above and re-run with --yes.`,
    undefined,
    2,
  );
}

export function noMintError(): RtpError {
  return new RtpError(
    `No mint specified. Use --mint <pubkey> or set defaultMint in config (rtp init).`,
    undefined,
    2,
  );
}
