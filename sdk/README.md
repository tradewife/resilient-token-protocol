# @resilient-protocol/sdk

Add RTP to your launch flow in one function call. Every token you launch gets an autonomous yield treasury.

## Install

```bash
npm install @resilient-protocol/sdk @solana/web3.js @solana/spl-token @coral-xyz/anchor
```

## Quick Start

### 1. Register your token with an autonomous treasury

```typescript
import { registerWithRTP, RTP_PROGRAM_ID } from "@resilient-protocol/sdk";
import { Connection, Keypair, PublicKey } from "@solana/web3.js";

const connection = new Connection("https://api.devnet.solana.com");
const payer = Keypair.generate(); // your launchpad's keypair

const result = await registerWithRTP(connection, payer, {
  mint: new PublicKey("YourTokenMint111111111111111111111111111111"),
  platform: "pumpfun",              // "pumpfun" | "bags" | "raydium"
  name: "Community Token",
  symbol: "CMTY",
  holdersWallet: payer.publicKey,    // optional, defaults to payer
  projectDevWallet: payer.publicKey, // optional, defaults to payer
  ecosystemWallet: payer.publicKey,  // optional, defaults to payer
  minRunwayBalance: 10_000_000,      // optional, defaults to 10_000_000
});

console.log("Mint:", result.mint);
console.log("Treasury PDA:", result.treasuryPDA);
console.log("Vault PDA:", result.vaultPDA);
console.log("Adopter PDA:", result.adopterPDA);
console.log("Explorer:", result.explorerUrl);
```

### 2. Read treasury state (for your token dashboard)

```typescript
import { fetchTreasuryState } from "@resilient-protocol/sdk";

const state = await fetchTreasuryState(connection, result.mint);
console.log("Phase:", state.phase);
console.log("Vault balance:", state.vaultBalance, "tokens");
console.log("Total distributed:", state.totalDistributedHolders);
```

### 3. Crank fee distribution (permissionless — anyone can call)

```typescript
import { withdrawAndRedistribute } from "@resilient-protocol/sdk";

const { withdrawSig, redistributeSig } = await withdrawAndRedistribute(
  connection,
  payer,
  result.mint,
);
console.log("Fees withdrawn:", withdrawSig);
if (redistributeSig) console.log("Redistributed:", redistributeSig);
```

## What Your Token Gets

- **Transfer fees route to a program-owned vault** — not a wallet anyone controls
- **An agent swarm trades yield strategies on Hyperliquid nightly** — validated by backtesting + WFA
- **Yield returns to the treasury** → redistributed 70/20/10 on-chain (holders / dev / ecosystem)
- **Phase evolution**: Sustenance → Ecosystem → Humanity (irreversible, threshold-gated)
- **The program enforces constraints** — no rug is possible by design

## Integration Checklist

1. **Call `registerWithRTP()` after your platform creates the mint** — your token now has an autonomous treasury
2. **Store the returned `treasuryPDA`** alongside your token record in your database
3. **(Optional)** Add `fetchTreasuryState()` to your token detail page to show treasury health

## Constants

| Export | Value |
|--------|-------|
| `RTP_PROGRAM_ID` | `8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB` |
| `RTP_DEVNET_RPC` | `https://api.devnet.solana.com` |
| `RTP_MAINNET_RPC` | `https://api.mainnet-beta.solana.com` |

## No RTP Token

There is no RTP token. RTP is infrastructure. It serves the tokens that adopt it.

## License

MIT
