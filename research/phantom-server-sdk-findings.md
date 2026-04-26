# Phantom Server SDK — Research Findings

**Date:** April 2026  
**Purpose:** Production security hardening spec for RTP's Trading Wing signing architecture  
**Status:** Research complete — recommendations pending implementation

---

## Table of Contents

1. [Server SDK Overview](#1-server-sdk-overview)
2. [Authentication Models](#2-authentication-models)
3. [MCP Server Session Auto-Refresh](#3-mcp-server-session-auto-refresh)
4. [Spending Limits](#4-spending-limits)
5. [MCP Server vs Server SDK — When to Use Which](#5-mcp-server-vs-server-sdk--when-to-use-which)
6. [EIP-712 Signing for Hyperliquid](#6-eip-712-signing-for-hyperliquid)
7. [Rust Binding Status](#7-rust-binding-status)
8. [PHANTOM_APP_ID Integration](#8-phantom_app_id-integration)
9. [Version Timeline (April 2026)](#9-version-timeline-april-2026)
10. [Recommendations for RTP](#10-recommendations-for-rtp)

---

## 1. Server SDK Overview

**Package:** `@phantom/server-sdk`  
**Source:** [github.com/phantom/phantom-connect-sdk](https://github.com/phantom/phantom-connect-sdk) (open-source, MIT license)  
**Dependencies:** `@phantom/client` (HTTP API), `@phantom/api-key-stamper` (request authentication)

The Server SDK enables programmatic wallet creation, message signing, and transaction signing/sending from backend servers — without browser-based OAuth flows.

```typescript
import { ServerSDK, NetworkId } from "@phantom/server-sdk";

const sdk = new ServerSDK({
  organizationId: process.env.ORGANIZATION_ID,
  appId: process.env.APP_ID,
  apiPrivateKey: process.env.PRIVATE_KEY,
  apiBaseUrl: process.env.API_URL,
});
```

### Creating Wallets

```typescript
const wallet = await sdk.createWallet("User Wallet");
// Returns: { walletId, addresses: [{ addressType, address }] }
```

### Signing Messages (Solana)

```typescript
const signature = await sdk.signMessage({
  walletId: wallet.walletId,
  message: "Hello from Phantom!",
  networkId: NetworkId.SOLANA_MAINNET,
});
```

### Signing & Sending Solana Transactions

```typescript
import { Transaction } from "@solana/web3.js";

const solanaTransaction = new Transaction().add(/* instructions */);
await sdk.signAndSendTransaction({
  walletId: wallet.walletId,
  transaction: solanaTransaction,
  networkId: NetworkId.SOLANA_MAINNET,
});
```

### Signing & Sending EVM Transactions

```typescript
const evmTransaction = {
  to: "0x742d35Cc6634C0532925a3b8D4C8db86fB5C4A7E",
  value: 1000000000000000000n,
  data: "0x",
};
await sdk.signAndSendTransaction({
  walletId: wallet.walletId,
  transaction: evmTransaction,
  networkId: NetworkId.ETHEREUM_MAINNET,
});
```

### Signing Raw Bytes / Hex Strings

```typescript
await sdk.signAndSendTransaction({
  walletId: wallet.walletId,
  transaction: "0x01020304",
  networkId: NetworkId.ETHEREUM_MAINNET,
});
```

### Key Methods Summary

| Method | Returns | Description |
|--------|---------|-------------|
| `sdk.createWallet(name)` | `{ walletId, addresses }` | Create a new embedded wallet |
| `sdk.signMessage({ walletId, message, networkId })` | `signature` | Sign a UTF-8 message |
| `sdk.signAndSendTransaction({ walletId, transaction, networkId })` | `tx result` | Sign and broadcast a transaction |

---

## 2. Authentication Models

Phantom has **two distinct auth paths** depending on the integration target:

### Server SDK (Backend / Headless)

Authenticates via API key stamping — no browser interaction required.

| Parameter | Source |
|-----------|--------|
| `organizationId` | Phantom Portal dashboard |
| `appId` | Phantom Portal dashboard |
| `apiPrivateKey` | API key for request stamping |

The `@phantom/api-key-stamper` package stamps every API request with a cryptographic signature derived from the private API key. This is the **non-interactive, headless-friendly** authentication path suitable for server deployments and cron jobs.

### MCP Server / CLI (Interactive Agent)

Authenticates via Phantom Connect — browser-based OAuth flow with Google/Apple/Phantom extension login.

- `phantom login` persists sessions locally at `~/.phantom-mcp/session.json`
- Tokens auto-refresh in the background (see [§3](#3-mcp-server-session-auto-refresh))
- Not suitable for fully headless server deployments without initial browser interaction

**Key takeaway for RTP:** The Server SDK's API-key auth is the correct path for production Trading Wing operations. The MCP server (current approach) is suitable for development and interactive agent use but requires browser-initiated session creation.

---

## 3. MCP Server Session Auto-Refresh

Yes — `phantom login` produces a long-lived session that auto-refreshes:

| Feature | Details |
|---------|---------|
| Session persistence | `~/.phantom-mcp/session.json` |
| Auth method | Phantom Connect (browser-based OAuth) |
| Token refresh | Automatic, background |
| MCP Server v1.0.3+ | Detects 401 errors, refreshes token, retries request |
| MCP Server v2.0.1 | Synchronous token refresh before API calls (prevents expired token reaching server) |

This means the MCP server subprocess approach (current RTP method) is operationally stable for long-running daemon processes, provided the initial session was established via `phantom login`.

---

## 4. Spending Limits

Spending limits are **on-chain enforced** transaction limits configured by users during the Phantom Connect flow.

| Aspect | Details |
|--------|---------|
| Configuration | User-controlled via Phantom wallet settings |
| Enforcement | Three-step: simulate tx → policy check → approve/reject |
| Scope | Per-app basis (different limits per dApp) |
| Error code | `SPENDING_LIMIT_EXCEEDED` |
| Enforcement level | KMS-level — checked before the transaction is signed |
| Availability | Currently for embedded wallet (Phantom Connect) flows only |

**Critical: spending limits are NOT programmatically settable by the app.** Users retain full control through Phantom wallet settings. The app can only detect the `SPENDING_LIMIT_EXCEEDED` error and handle it gracefully.

**RTP implication:** The Trading Wing must handle `SPENDING_LIMIT_EXCEEDED` errors in its order execution path. For production, ensure the Phantom Connect flow communicates expected trading volume to users during wallet setup.

---

## 5. MCP Server vs Server SDK — When to Use Which

| Dimension | MCP Server (`@phantom/mcp-server`) | Server SDK (`@phantom/server-sdk`) |
|-----------|--------------------------------------|-------------------------------------|
| **Target** | AI agents (Claude, Cursor, etc.) | Backend server applications |
| **Protocol** | MCP (stdio JSON-RPC) | Direct HTTP API calls |
| **Wallet** | Dedicated agent wallet (separate from personal) | Programmatically created wallets |
| **Auth** | Phantom Connect browser session | API key stamping (non-interactive) |
| **Tools** | 29 tools (swap, perps, transfers, signing) | Core methods (create, sign, send) |
| **Swap fees** | No fees | Standard |
| **Per-token isolation** | `derivationIndex` parameter | Separate wallet creation per token |
| **Setup** | `phantom login` (browser) | API keys from Portal (headless) |

**Current RTP path:** MCP Server subprocess with `derivationIndex` per-token isolation.  
**Recommended production path:** Server SDK for headless Trading Wing execution, MCP server retained for interactive demo/debugging.

---

## 6. EIP-712 Signing for Hyperliquid

EIP-712 typed data signing is available through **multiple paths**, with different coverage:

| Path | EIP-712 Support | Notes |
|------|-----------------|-------|
| MCP Server `sign_evm_typed_data` | ✅ Full | Works for HL testnet; some perps write ops return 403 |
| React SDK `useEthereum.signTypedData` | ✅ Full | Client-side only |
| Server SDK `signAndSendTransaction` | ⚠️ Partial | Supports EVM transactions; EIP-712 typed data not explicitly documented |
| Direct ETH keypair | ✅ Full | Current RTP path via `configs/hl_testnet_key.json` |

### Current RTP Signing Architecture

```
HL order signing: ETH keypair directly (EIP-712)
   └── configs/hl_testnet_key.json

MCP bridge signing: Phantom MCP server subprocess
   └── @phantom/mcp-server, fee-free swaps, Relay cross-chain bridge

Per-token isolation: derivationIndex parameter
   └── Index 0: default agent, 1: Token A, 2: Token B, ...

Solana CPI signing: Phantom KMS (production) → local devnet keypair (demo)
```

**Gap:** The Server SDK README does not explicitly show an EIP-712 `signTypedData` method. For HL order construction, the current direct-keypair approach works. For production, the MCP server's `sign_evm_typed_data` tool or manual EIP-712 construction over the Server SDK's raw signing path would be needed.

---

## 7. Rust Binding Status

**No Rust binding exists.** The SDK is TypeScript-only (98.6% TypeScript, 1.4% JavaScript).

Dependency chain:
```
@phantom/server-sdk
  └── @phantom/client
        └── @phantom/api-key-stamper
```

### Options for Rust Integration

| Option | Complexity | Maintenance | Headless |
|--------|-----------|-------------|----------|
| **1. Server SDK via Node.js subprocess** | Low | Low | ✅ Yes |
| **2. `@phantom/cli` subprocess (`phantom --mcp`)** | Low | Low | ❌ Needs browser init |
| **3. Reimplement HTTP API + request stamping in Rust** | High | High | ✅ Yes |
| **4. Keep MCP server subprocess (current approach)** | Low | Low | ❌ Needs browser init |

**Recommendation:** Option 1 (Server SDK via Node.js subprocess) for production Trading Wing. This replaces the MCP server subprocess with a lighter Node.js script that uses API-key auth — no browser session required, suitable for `rtp-daemon` cron cycles.

---

## 8. PHANTOM_APP_ID Integration

RTP's registered app ID: `2fbef7dc-7975-4378-ba2b-ff8018ad2325`

| Component | Uses PHANTOM_APP_ID? | Details |
|-----------|---------------------|---------|
| React SDK `PhantomProvider` | ✅ Yes | Config parameter |
| Server SDK | ✅ Yes | `appId` constructor parameter |
| MCP Server | ❌ No | Removed in Apr 22 session |
| OpenClaw plugin | ✅ Yes | Accepts `PHANTOM_APP_ID` + `PHANTOM_CLIENT_ID` in config |
| CLI | ✅ Optional | Can attribute tool calls with app ID |

---

## 9. Version Timeline (April 2026)

| Date | Release | Highlights |
|------|---------|------------|
| Early April | MCP Server v0.2.4, SDK v1.0.7 | 13 tools, dapp-sponsored transactions |
| Apr 7 | SDK v2.0.0-beta.0 | OAuth2 PKCE support |
| Apr 10–15 | MCP Server v1.0.0 → v1.0.4 | 28–29 tools, stable release, perps, simulation |
| Apr 15 | CLI v1.0.0, MCP v1.1.0 | CLI as unified architecture |
| Apr 18 | MCP v1.0.3+ | Auto session refresh on 401 |
| Apr 20 | SDK v2.0.1 STABLE, CLI v1.2, MCP v1.2 | Strict validation, CLI-powered |
| Apr 21 | CLI v1.2.5 | Latest release at time of research |

---

## 10. Recommendations for RTP

### Production Signing Path

1. **Migrate Trading Wing signing to Server SDK** — API-key auth is headless, no browser session dependency. Create a thin Node.js subprocess wrapper around `@phantom/server-sdk` that the Rust Trading Wing calls.

2. **Retain MCP server for interactive use** — Keep `phantom_mcp.rs` for demo flows, debugging, and manual intervention. The MCP server's 29-tool surface is ideal for interactive agent work.

3. **Keep direct ETH keypair for HL EIP-712** — The current `configs/hl_testnet_key.json` path is battle-tested. The Server SDK's EIP-712 coverage is unclear; don't migrate HL signing until EIP-712 typed data is explicitly documented.

### Security Hardening

4. **Handle `SPENDING_LIMIT_EXCEEDED`** — Add error handling in the Trading Wing's order execution path. Surface meaningful error messages to the Coordinator for audit logging.

5. **Per-token wallet isolation** — The MCP server's `derivationIndex` approach works for development. For production with the Server SDK, create separate wallets per registered token (each gets unique `walletId`).

6. **API key storage** — Server SDK's `apiPrivateKey` must be stored in environment variables or a secrets manager, never in config files. Add to `configs/` gitignore.

### Migration Priority

| Priority | Task | Effort |
|----------|------|--------|
| P0 | Add `SPENDING_LIMIT_EXCEEDED` handling to Trading Wing | Low |
| P1 | Server SDK subprocess wrapper for headless signing | Medium |
| P2 | Migrate HL EIP-712 signing to Server SDK (pending EIP-712 docs) | High |
| P3 | Replace MCP subprocess with Server SDK in `rtp-daemon` | Medium |

---

*Research conducted April 2026. Versions and APIs subject to change — verify against latest [Phantom Connect SDK](https://github.com/phantom/phantom-connect-sdk) before implementation.*
