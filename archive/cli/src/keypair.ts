// RTP CLI — Keypair loading and display utilities.

import { Keypair, PublicKey } from "@solana/web3.js";
import fs from "fs";

export function loadKeypair(filePath: string): Keypair {
  if (!fs.existsSync(filePath)) {
    throw new Error(`Keypair file not found: ${filePath}`);
  }
  const content = fs.readFileSync(filePath, "utf-8");
  let bytes: number[];
  try {
    bytes = JSON.parse(content);
  } catch {
    throw new Error(`Invalid keypair JSON: ${filePath}`);
  }
  if (!Array.isArray(bytes) || bytes.length < 32) {
    throw new Error(`Invalid keypair format: ${filePath}`);
  }
  return Keypair.fromSecretKey(new Uint8Array(bytes));
}

export function truncatePubkey(pubkey: PublicKey | string): string {
  const s = typeof pubkey === "string" ? pubkey : pubkey.toBase58();
  if (s.length <= 12) return s;
  return `${s.slice(0, 6)}...${s.slice(-4)}`;
}

export function formatSol(lamports: number): string {
  return (lamports / 1e9).toFixed(4);
}

export function formatSolFull(lamports: number): string {
  return (lamports / 1e9).toFixed(9);
}

export function isHotWallet(filePath: string): boolean {
  const resolved = filePath.replace(process.env.HOME ?? "~", "~");
  return resolved.includes("~/.config/solana/") || resolved.endsWith(".json");
}
