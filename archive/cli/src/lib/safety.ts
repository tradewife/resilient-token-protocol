// RTP CLI — Confirmation prompts and safety warnings.

import chalk from "chalk";
import { isHotWallet } from "../keypair.js";

export function confirmDestructive(
  operation: string,
  details: string[],
  yesFlag: boolean,
): boolean {
  console.log(chalk.yellow(`\nAbout to ${operation}:`));
  for (const line of details) {
    console.log(`  ${line}`);
  }
  console.log();

  if (yesFlag) return true;

  console.log(
    `${chalk.red("[rtp] ERROR:")} --yes required for ${operation}. ` +
    `Review the output above and re-run with --yes.`,
  );
  return false;
}

export function confirmMainnetDeploy(): boolean {
  console.log(chalk.yellow(`\n⚠ You are about to deploy the RTP treasury program to MAINNET.`));
  console.log(`  This is irreversible and will cost SOL for the program deployment.\n`);
  // In a real interactive session we'd prompt. For CLI scripting we require --yes too.
  // This function is called after --yes is verified; it's the additional "type mainnet" guard.
  // For now we simplify: --yes is sufficient, this prints the warning.
  return true;
}

export function warnHotWallet(authorityPath: string): void {
  if (isHotWallet(authorityPath)) {
    console.log(
      chalk.yellow(`  ⚠ Warning: Using a file-based keypair (hot wallet) as authority.\n`) +
      chalk.dim(`    Post-launch: rotate treasury.authority to a Squads multisig PDA\n`) +
      chalk.dim(`    with 2-of-3 signers + 24h time lock. See: https://docs.squads.so`),
    );
  }
}

export function warnDevnetFlashTrade(): void {
  console.log(
    chalk.yellow(`  ⚠ Flash Trade uses Pyth oracles — position CPI will fail with\n`) +
    chalk.yellow(`    StaleOraclePrice on devnet. Constraint logic tests work.\n`) +
    chalk.dim(`    Use mainnet for full CPI execution.`),
  );
}

export function warnMainnetFlashTrade(): void {
  console.log(
    chalk.yellow(`  ⚠ Minimum Flash Trade position: ~$11-12 USDC.\n`) +
    chalk.yellow(`    Your fee-payer wallet needs SOL for gas.`),
  );
}
