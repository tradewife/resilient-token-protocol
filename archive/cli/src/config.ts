// RTP CLI — Configuration loading and resolution.

import fs from "fs";
import path from "path";
import os from "os";

export interface RtpConfig {
  cluster: "devnet" | "mainnet";
  feePayerKeypairPath: string;
  authorityKeypairPath: string;
  defaultMint: string | null;
  defaultAuthority: string | null;
  rpcUrl: string | null;
  railwayTokenPath: string | null;
  nightResultsDir: string;
}

const GLOBAL_CONFIG_PATH = path.join(os.homedir(), ".rtp", "config.json");
const LOCAL_CONFIG_PATH = ".rtp.json";

const DEFAULTS: RtpConfig = {
  cluster: "devnet",
  feePayerKeypairPath: path.join(os.homedir(), ".config", "solana", "id.json"),
  authorityKeypairPath: path.join(os.homedir(), ".config", "solana", "id.json"),
  defaultMint: null,
  defaultAuthority: null,
  rpcUrl: null,
  railwayTokenPath: null,
  nightResultsDir: "./data/night_results",
};

function readJsonFile(filePath: string): Partial<RtpConfig> | null {
  try {
    if (!fs.existsSync(filePath)) return null;
    const raw = fs.readFileSync(filePath, "utf-8");
    return JSON.parse(raw) as Partial<RtpConfig>;
  } catch {
    return null;
  }
}

export function loadConfig(): RtpConfig {
  const global = readJsonFile(GLOBAL_CONFIG_PATH) ?? {};
  const local = readJsonFile(LOCAL_CONFIG_PATH) ?? {};
  // Local overrides global; both override defaults.
  return { ...DEFAULTS, ...global, ...local };
}

export function saveConfig(config: RtpConfig, target: "global" | "local" = "global"): void {
  const filePath = target === "global" ? GLOBAL_CONFIG_PATH : LOCAL_CONFIG_PATH;
  const dir = path.dirname(filePath);
  if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(filePath, JSON.stringify(config, null, 2) + "\n", "utf-8");
}

export function resolveRpcUrl(config: RtpConfig): string {
  const envRpc = process.env.SOLANA_RPC_URL;
  if (envRpc) return envRpc;
  if (config.rpcUrl) return config.rpcUrl;
  switch (config.cluster) {
    case "mainnet": return "https://api.mainnet-beta.solana.com";
    case "devnet": return "https://api.devnet.solana.com";
  }
}

export function resolveKeypair(
  flagPath: string | undefined,
  envVar: string,
  configPath: string,
): string {
  if (flagPath) return flagPath;
  const envVal = process.env[envVar];
  if (envVal) return envVal;
  if (configPath && fs.existsSync(configPath)) return configPath;
  const solanaDefault = path.join(os.homedir(), ".config", "solana", "id.json");
  if (fs.existsSync(solanaDefault)) return solanaDefault;
  throw new Error(
    `No keypair found. Set --authority flag, ${envVar} env var, or run 'rtp init'.`,
  );
}

export function resolveMint(flagMint: string | undefined, config: RtpConfig): string {
  if (flagMint) return flagMint;
  if (config.defaultMint) return config.defaultMint;
  throw new Error(
    "No mint specified. Use --mint <pubkey> or set defaultMint in config (rtp init).",
  );
}

export const GLOBAL_CONFIG_PATH_EXPORT = GLOBAL_CONFIG_PATH;
