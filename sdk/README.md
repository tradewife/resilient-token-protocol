# @resilient-protocol/sdk

Launchpad SDK for the Resilient Token Protocol. One function — create a Token-2022 mint whose transfer fees permanently route to the RTP treasury vault.

## Install

```bash
npm install @resilient-protocol/sdk @solana/web3.js @solana/spl-token
```

## Usage

```typescript
import { createRTPToken, RTP_TREASURY_VAULT } from "@resilient-protocol/sdk";
import { Connection, Keypair } from "@solana/web3.js";

const connection = new Connection("https://api.devnet.solana.com");
const payer = Keypair.generate(); // your launchpad's keypair

const result = await createRTPToken(connection, payer, {
  name: "My Launchpad Token",
  symbol: "MLT",
  supply: 1_000_000_000,  // 1 billion tokens
  feeBps: 200,            // 2% transfer fee → RTP treasury
});

console.log("Mint:", result.mint);
console.log("TX:", result.explorerUrl);
console.log("Fee destination:", result.treasuryVault);
// Fee destination is always FNQbK1Vw77aT7qM1EMSmeEPDGizSNhX4rkkYBKQNFotF
```

## Constants

| Export | Value |
|--------|-------|
| `RTP_TREASURY_VAULT` | `FNQbK1Vw77aT7qM1EMSmeEPDGizSNhX4rkkYBKQNFotF` |
| `RTP_DEVNET_RPC` | `https://api.devnet.solana.com` |
| `RTP_MAINNET_RPC` | `https://api.mainnet-beta.solana.com` |

## What happens

1. A Token-2022 mint is created with a `TransferFeeConfig` extension
2. The fee destination is **hardcoded** to the RTP treasury vault — launchpads cannot redirect fees
3. Initial supply is minted to the launchpad's associated token account
4. Every transfer of this token generates fees that flow to the RTP treasury
5. The treasury autonomously generates yield and redistributes to holders (70/20/10 split)

## License

MIT
