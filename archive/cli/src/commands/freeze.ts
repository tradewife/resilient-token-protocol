// RTP CLI — rtp freeze / rtp unfreeze: Emergency halt and resume.
// Uses SDK's freezeTreasury and unfreezeTreasury directly.

import { Command } from "commander";

import { loadConfig, resolveKeypair } from "../config.js";
import { loadKeypair, truncatePubkey, formatSol } from "../keypair.js";
import { printOk, printInfo, printNote, getOutputMode } from "../format.js";
import { printError } from "../errors.js";
import { createConnection, explorerTxUrl } from "../lib/rpc.js";
import { confirmDestructive, warnHotWallet } from "../lib/safety.js";

export function makeFreezeCommand(): Command {
  const cmd = new Command("freeze")
    .description("Emergency freeze — halt all treasury operations")
    .requiredOption("--authority <pubkey>", "Treasury authority address")
    .requiredOption("--authority-keypair <path>", "Authority keypair path")
    .option("--cluster <cluster>", "Cluster (devnet|mainnet)", "devnet")
    .option("--yes", "Confirm destructive operation")
    .option("--json", "JSON output")
    .option("--quiet", "Suppress output except errors")
    .action(async (opts) => {
      const mode = getOutputMode(opts);
      try {
        const config = loadConfig();
        const authorityPath = resolveKeypair(opts.authorityKeypair, "AUTHORITY_KEYPAIR_PATH", config.authorityKeypairPath);
        const authority = loadKeypair(authorityPath);
        const connection = createConnection(config);
        const PublicKey = (await import("@solana/web3.js")).PublicKey;
        const authorityPk = new PublicKey(opts.authority);

        // Fetch current state for display
        const sdk = await import("../../../sdk/index.ts");
        const state = await sdk.fetchTreasuryState(connection, authorityPk);

        if (!confirmDestructive(
          "FREEZE treasury",
          [
            `Authority:     ${truncatePubkey(opts.authority)}`,
            `Balance:       ${formatSol(state.solBalance)} SOL`,
            `Frozen:        ${state.isFrozen ? "YES" : "NO → YES"}`,
            ``,
            `This will block ALL state-mutating instructions.`,
            `To resume: rtp unfreeze --authority <pubkey> --authority-keypair <path>`,
          ],
          opts.yes,
        )) {
          process.exit(2);
        }

        warnHotWallet(authorityPath);

        const result = await sdk.freezeTreasury(connection, authority, authorityPk);

        if (mode === "json") {
          console.log(JSON.stringify({ authority: opts.authority, frozen: true, ...result }, null, 2));
          return;
        }

        printOk("Treasury FROZEN.");
        if (result.signature) {
          printInfo(`TX: ${explorerTxUrl(result.signature, opts.cluster)}`);
        }
        printNote(`To resume: rtp unfreeze --authority ${truncatePubkey(opts.authority)} --authority-keypair <path>`);
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
    .requiredOption("--authority <pubkey>", "Treasury authority address")
    .requiredOption("--authority-keypair <path>", "Authority keypair path")
    .option("--cluster <cluster>", "Cluster (devnet|mainnet)", "devnet")
    .option("--yes", "Confirm operation")
    .option("--json", "JSON output")
    .option("--quiet", "Suppress output except errors")
    .action(async (opts) => {
      const mode = getOutputMode(opts);
      try {
        const config = loadConfig();
        const authorityPath = resolveKeypair(opts.authorityKeypair, "AUTHORITY_KEYPAIR_PATH", config.authorityKeypairPath);
        const authority = loadKeypair(authorityPath);
        const connection = createConnection(config);
        const PublicKey = (await import("@solana/web3.js")).PublicKey;
        const authorityPk = new PublicKey(opts.authority);

        if (!confirmDestructive(
          "UNFREEZE treasury",
          [
            `Authority:     ${truncatePubkey(opts.authority)}`,
            `Frozen:        YES → NO`,
            ``,
            `Operations will resume.`,
          ],
          opts.yes,
        )) {
          process.exit(2);
        }

        warnHotWallet(authorityPath);

        const sdk = await import("../../../sdk/index.ts");
        const result = await sdk.unfreezeTreasury(connection, authority, authorityPk);

        if (mode === "json") {
          console.log(JSON.stringify({ authority: opts.authority, frozen: false, ...result }, null, 2));
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
