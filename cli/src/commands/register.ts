// RTP CLI — rtp register: Register adopter or strategy.

import { Command } from "commander";

import { loadConfig, resolveKeypair } from "../config.js";
import { loadKeypair, truncatePubkey } from "../keypair.js";
import { printOk, printInfo, getOutputMode } from "../format.js";
import { printError } from "../errors.js";
import { createConnection, explorerTxUrl } from "../lib/rpc.js";
import { warnHotWallet } from "../lib/safety.js";

export function makeRegisterCommand(): Command {
  const cmd = new Command("register")
    .description("Register adopter or strategy");

  // register adopter — permissionless (any signer)
  cmd.addCommand(
    new Command("adopter")
      .description("Register an adopter record on-chain (permissionless)")
      .requiredOption("--authority <pubkey>", "Treasury authority address")
      .option("--authority-keypair <path>", "Signer keypair path (fee-payer, not authority-gated)")
      .option("--adopter-id <string>", "Optional adopter ID string (defaults to authority)")
      .option("--beta", "Use beta adopter registration")
      .option("--cluster <cluster>", "Cluster (devnet|mainnet)", "devnet")
      .option("--json", "JSON output")
      .option("--quiet", "Suppress output except errors")
      .action(async (opts) => {
        const mode = getOutputMode(opts);
        try {
          const config = loadConfig();
          const keypairPath = resolveKeypair(opts.authorityKeypair, "KEYPAIR_PATH", config.feePayerKeypairPath);
          const signer = loadKeypair(keypairPath);
          const connection = createConnection(config);
          const PublicKey = (await import("@solana/web3.js")).PublicKey;
          const authorityPk = new PublicKey(opts.authority);

          if (mode !== "quiet") {
            printInfo(`Registering ${opts.beta ? "beta " : ""}adopter for authority: ${truncatePubkey(opts.authority)}`);
            printInfo(`Signer: ${truncatePubkey(signer.publicKey)}`);
          }

          const sdk = await import("../../../sdk/index.ts");

          // In the native-SOL model, the treasury is keyed on authority.
          // The adopterId is the caller's chosen identifier (e.g., token mint or project name).
          const authorityPubkey = signer.publicKey; // fee-payer / signer
          const adopterId = opts.adopterId ?? opts.authority;

          if (opts.beta) {
            const result = await sdk.registerAdopterBeta(
              connection,
              signer,
              authorityPk,
              adopterId,
              Math.floor(Date.now() / 1000) + 90 * 24 * 3600,
            );
            if (mode === "json") {
              console.log(JSON.stringify(result, null, 2));
              return;
            }
            printOk("Beta adopter registered!");
          } else {
            // Permanent adopter registration via registerWithRTP (initialize + register_adopter).
            printInfo("Using registerWithRTP (initialize + first adopter)");
            const result = await sdk.registerWithRTP(connection, signer, { authority: authorityPk });
            if (mode === "json") {
              console.log(JSON.stringify(result, null, 2));
              return;
            }
            printOk("Adopter registered!");
          }
        } catch (e) {
          printError(e);
          process.exit(1);
        }
      }),
  );

  // register strategy — authority-gated
  cmd.addCommand(
    new Command("strategy")
      .description("Promote a strategy to Live status (authority-gated)")
      .requiredOption("--config <json-file>", "Path to strategy config JSON")
      .requiredOption("--authority <path>", "Authority keypair path")
      .option("--authority-pubkey <pubkey>", "Treasury authority address (derived from authority keypair if omitted)")
      .option("--cluster <cluster>", "Cluster (devnet|mainnet)", "devnet")
      .option("--json", "JSON output")
      .option("--quiet", "Suppress output except errors")
      .action(async (opts) => {
        const mode = getOutputMode(opts);
        try {
          const config = loadConfig();
          const authorityPath = resolveKeypair(opts.authority, "AUTHORITY_KEYPAIR_PATH", config.authorityKeypairPath);
          const authority = loadKeypair(authorityPath);
          const connection = createConnection(config);

          const fs = await import("fs");
          const strategyConfig = JSON.parse(fs.readFileSync(opts.config, "utf-8"));
          const PublicKey = (await import("@solana/web3.js")).PublicKey;

          const authorityPk = opts.authorityPubkey
            ? new PublicKey(opts.authorityPubkey)
            : authority.publicKey;

          if (mode !== "quiet") {
            printInfo(`Promoting strategy: ${strategyConfig.strategyId ?? strategyConfig.id ?? "from config"}`);
            warnHotWallet(authorityPath);
          }

          const sdk = await import("../../../sdk/index.ts");

          const result = await sdk.registerStrategy(
            connection,
            authority,
            authorityPk.toBase58(),
            strategyConfig.strategyId ?? strategyConfig.id,
            strategyConfig.promotionSharpeX100 ?? Math.round((strategyConfig.oosSharpe ?? 0) * 100),
          );

          if (mode === "json") {
            console.log(JSON.stringify(result, null, 2));
            return;
          }

          printOk(`Strategy promoted!`);
          if (result?.signature) {
            printInfo(`TX: ${explorerTxUrl(result.signature, opts.cluster)}`);
          }
        } catch (e) {
          printError(e);
          process.exit(1);
        }
      }),
  );

  return cmd;
}
