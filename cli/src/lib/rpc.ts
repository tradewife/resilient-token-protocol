// RTP CLI — RPC connection helper.

import { Connection } from "@solana/web3.js";
import { loadConfig, resolveRpcUrl, type RtpConfig } from "../config.js";

export function createConnection(config?: RtpConfig): Connection {
  const cfg = config ?? loadConfig();
  const rpcUrl = resolveRpcUrl(cfg);
  return new Connection(rpcUrl, "confirmed");
}

export function clusterLabel(cluster: string): string {
  switch (cluster) {
    case "mainnet": return "mainnet-beta";
    case "devnet": return "devnet";
    default: return cluster;
  }
}

export function explorerTxUrl(signature: string, cluster: string): string {
  return `https://explorer.solana.com/tx/${signature}?cluster=${clusterLabel(cluster)}`;
}
