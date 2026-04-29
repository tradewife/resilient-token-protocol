// RTP CLI — rtp freeze / rtp unfreeze: Emergency halt and resume.

import { Command } from "commander";

import { loadConfig, resolveMint, resolveKeypair } from "../config.js";
import { loadKeypair, truncatePubkey, formatSol } from "../keypair.js";
import { printOk, printInfo, printNote, getOutputMode } from "../format.js";
import { printError } from "../errors.js";
import { createConnection, explorerTxUrl } from "../lib/rpc.js";
import { confirmDestructive, warnHotWallet } from "../lib/safety.js";

export function makeFreezeCommand(): Command {
  const cmd = new Command("freeze")
    .description("Emergency freeze — halt all treasury operations")
    .requiredOption("--mint <pubkey>", "Token mint address")
    .requiredOption("--authority <path>", "Authority keypair path")
    .option("--cluster <cluster>", "Cluster (devnet|mainnet)", "devnet")
    .option("--yes", "Confirm destructive operation")
    .option("--json", "JSON output")
    .option("--quiet", "Suppress output except errors")
    .action(async (opts) => {
      const mode = getOutputMode(opts);
      try {
        const config = loadConfig();
        const mint = resolveMint(opts.mint, config);
        const authorityPath = resolveKeypair(opts.authority, "AUTHORITY_KEYPAIR_PATH", config.authorityKeypairPath);
        const authority = loadKeypair(authorityPath);
        const connection = createConnection(config);
        const PublicKey = (await import("@solana/web3.js")).PublicKey;
        const mintPk = new PublicKey(mint);

        // Fetch current state for display
        const sdk = await import("../../../sdk/index.ts");
        const state = await sdk.fetchTreasuryState(connection, mintPk);

        if (!confirmDestructive(
          "FREEZE treasury",
          [
            `Mint:          ${truncatePubkey(mint)}`,
            `Vault balance: ${formatSol(state.vaultBalance)} SOL`,
            `Frozen:        ${state.isFrozen ? "YES" : "NO → YES"}`,
            ``,
            `This will block ALL 15 state-mutating instructions.`,
            `To resume: rtp unfreeze --mint ${truncatePubkey(mint)} --authority <path>`,
          ],
          opts.yes,
        )) {
          process.exit(2);
        }

        warnHotWallet(authorityPath);

        const { exportFreezeTreasury } = await import("../../../scripts/emergency-freeze.ts");
        const result = await exportFreezeTreasury(connection, authority, mintPk);

        if (mode === "json") {
          console.log(JSON.stringify({ mint, frozen: true, ...result }, null, 2));
          return;
        }

        printOk("Treasury FROZEN.");
        if (result.signature) {
          printInfo(`TX: ${explorerTxUrl(result.signature, opts.cluster)}`);
        }
        printNote(`To resume: rtp unfreeze --mint ${truncatePubkey(mint)} --authority <path>`);
      } catch (e) {
        printError(e);
        process.exit(1);
      }
    });

  return cmd;
}

export function makeUnfreezeCommand(): Command {
  return new Command("unfreeze")
    .description("Resume operations — unfreeze treasury")
    .requiredOption("--mint <pubkey>", "Token mint address")
    .requiredOption("--authority <path>", "Authority keypair path")
    .option("--cluster <cluster>", "Cluster (devnet|mainnet)", "devnet")
    .option("--yes", "Confirm operation")
    .option("--json", "JSON output")
    .option("--quiet", "Suppress output except errors")
    .action(async (opts) => {
      const mode = getOutputMode(opts);
      try {
        const config = loadConfig();
        const mint = resolveMint(opts.mint, config);
        const authorityPath = resolveKeypair(opts.authority, "AUTHORITY_KEYPAIR_PATH", config.authorityKeypairPath);
        const authority = loadKeypair(authorityPath);
        const connection = createConnection(config);
        const PublicKey = (await import("@solana/web3.js")).PublicKey;
        const mintPk = new PublicKey(mint);

        if (!confirmDestructive(
          "UNFREEZE treasury",
          [
            `Mint:          ${truncatePubkey(mint)}`,
            `Frozen:        YES → NO`,
            ``,
            `Operations will resume.`,
          ],
          opts.yes,
        )) {
          process.exit(2);
        }

        warnHotWallet(authorityPath);

        const { exportUnfreezeTreasury } = await import("../../../scripts/emergency-freeze.ts");
        const result = await exportUnfreezeTreasury(connection, authority, mintPk);

        if (mode === "json") {
          console.log(JSON.stringify({ mint, frozen: false, ...result }, null, 2));
          return;
        }

        printOk("Treasury UNFROZEN — operations resumed.");
        if (result.signature) {
          printInfo(`TX: ${explorerTxUrl(result.signature, opts.cluster)}`);
        }
      } catch (e) {
        printError(e);
        process.exit(1);
      }
    });
}
