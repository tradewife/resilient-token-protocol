# Multi-Platform Launcher Plan

## Context (read this first)

- **Project**: Resilient Token Protocol (RTP) — B2B yield treasury infrastructure for Solana launch platforms. There is NO RTP token.
- **Site**: Next.js 16 app at `dashboard/`. Public URL: resilientprotocol.xyz
- **Key docs**: Read `CLAUDE.md` for full project context. Read `dashboard/AGENTS.md` for Next.js version warnings.
- **Current pages**: `/` (dashboard), `/docs` (SDK guide), `/launch` (platform integration preview), `/research` (night shift results)
- **The /launch page already works**: RTP Direct mode creates Token-2022 mints with TransferFeeConfig on devnet via Phantom wallet. This plan adds 3 more platform options.
- **Design system**: All CSS is in `dashboard/src/app/globals.css`. Use existing classes (`.btn-launch`, `.btn-connect`, `.code-block`, `.form-input`, etc.). No new CSS frameworks.
- **Build check**: Always run `cd dashboard && npm run build` after changes.
- **Hackathon**: SWARMs/Canteen × Colosseum, deadline May 11, 2026. Metaplex is a sponsor.

## Goal

Add a platform selector to `/launch` so users can choose where to deploy their token.
Four platforms: **Metaplex**, **Pump.fun**, **Bags.fm**, **RTP Direct**.

Each platform mode generates a complete, copy-pasteable code snippet.
Only RTP Direct executes in-browser (current working flow on devnet).
The other three generate scripts the user runs themselves.

## Platforms

### 1. RTP Direct (current flow — Token-2022)
- **What**: `createRTPToken()` — Token-2022 mint with TransferFeeConfig → per-mint treasury PDA
- **Form fields**: name, symbol, supply, feeBps
- **Code**: existing `@resilient-protocol/sdk` snippet
- **Execution**: Works now — creates token on devnet via Phantom wallet
- **Status**: DONE

### 2. Metaplex Genesis (hackathon sponsor)
- **What**: Metaplex Genesis fair launch — SPL token via Genesis Launch Pool
- **API**: REST API at `https://api.metaplex.com` + Genesis SDK (`@metaplex-foundation/mpl-genesis`)
- **Docs**: https://www.metaplex.com/docs/smart-contracts/genesis/integration-apis/create-launch
- **Flow**:
  1. Create Genesis account + mint
  2. Create Launch Pool (fair launch / auction / presale)
  3. Register launch
- **Form fields**: name, symbol, supply, launch type (fair launch/auction/presale), pricing
- **Code generation**: Full Genesis SDK script
- **RTP integration**: After Genesis creates the mint, initialize RTP treasury via Anchor instruction
- **Token standard**: Standard SPL (not Token-2022) — same tension as Pump.fun
- **Hackathon angle**: Metaplex is a Colosseum sponsor — using their API directly is a strong signal

### 3. Pump.fun (largest memecoin launcher)
- **What**: Create token on Pump.fun bonding curve via PumpPortal API
- **API**: `https://pumpportal.fun/api/trade-local` (no API key needed for local tx)
- **Docs**: https://pumpportal.fun/creation/
- **Flow**:
  1. Upload metadata to IPFS (Pinata)
  2. POST to pumpportal.fun/api/trade-local with action "create"
  3. Get back serialized VersionedTransaction
  4. Sign with mint keypair + user wallet
  5. Send to Solana
- **Form fields**: name, symbol, description, image URL, website, twitter, telegram, devBuyAmount (SOL)
- **Code generation**: Full PumpPortal local transaction script
- **RTP integration**: After Pump.fun creates the mint, initialize RTP treasury
- **Token standard**: Standard SPL, bonding curve
- **Note**: Token-2022 with TransferFeeConfig is NOT compatible with Pump.fun's bonding curve. Dual-mode: Pump.fun creates SPL, then separate RTP treasury init.

### 4. Bags.fm (fee sharing aligns with RTP)
- **What**: Create token on Bags.fm with built-in fee sharing
- **API**: `https://public-api-v2.bags.fm/api/v1/token-launch/*` — requires API key
- **SDK**: `@bagsfm/bags-sdk` (TypeScript, well-documented)
- **Docs**: https://docs.bags.fm/how-to-guides/launch-token
- **Flow**:
  1. Create token info + metadata (IPFS upload handled by API)
  2. Create fee share config (creator gets fees, optional fee claimers)
  3. Create launch transaction
  4. Sign and send (Jito bundles supported)
- **Form fields**: name, symbol, description, image URL, website, twitter, telegram, initialBuyAmount (SOL), feeClaimers (optional)
- **Code generation**: Full Bags SDK script with fee sharing
- **RTP integration**: Set fee claimer to RTP treasury PDA — fees route to treasury automatically
- **Token standard**: Standard SPL on Meteora DLMM
- **Best RTP fit**: Bags' fee sharing model can route creator fees to the RTP treasury by default. The partner config system is literally designed for what we need.

## UI Design

### Platform Selector
Horizontal row of 4 cards above the form. Each card shows:
- Platform name + logo/color
- One-line description
- Selected state (border highlight)

Colors:
- RTP Direct: coral (our brand)
- Metaplex: #14F195 (Metaplex green)
- Pump.fun: #00d18c (Pump green)
- Bags.fm: #7C3AED (Bags purple)

### Form
Shared fields at top (name, symbol), then platform-specific fields below.
Platform-specific fields appear/disappear based on selection.

### Code Output
Below the form: generated code block with copy button.
Different snippet per platform.

### For RTP Direct only
The existing execute flow: Confirm → Sign with Phantom → Success screen with explorer links.

## Implementation Steps

1. **Add platform state and types** to `launch/page.tsx`
2. **Build PlatformSelector component** — 4 clickable cards
3. **Add platform-specific form fields** — conditional rendering
4. **Write code snippet generators** — one function per platform
5. **Wire up existing RTP Direct flow** to new structure (preserve all current functionality)
6. **Build and test** — `npm run build` must pass

## Key Files to Modify
- `dashboard/src/app/launch/page.tsx` — major rewrite (platform selector + multi-mode)

## Key Files to Read First
- `dashboard/src/app/launch/page.tsx` — current working RTP Direct flow
- `dashboard/src/app/docs/page.tsx` — copy-to-clipboard pattern, code block styling
- `dashboard/src/app/globals.css` — existing CSS classes to reuse

## Constraints
- Match existing visual style (globals.css classes)
- No new CSS frameworks
- All platform code is client-side code generation (no backend API calls for non-RTP platforms)
- RTP Direct remains fully functional with wallet adapter
- Must pass `npm run build`

## Research Resources
- Metaplex Genesis API: https://www.metaplex.com/docs/smart-contracts/genesis/integration-apis/create-launch
- Metaplex Genesis SDK: https://www.metaplex.com/docs/smart-contracts/genesis/sdk/api-client
- PumpPortal create token: https://pumpportal.fun/creation/
- Pump.fun SDK: https://github.com/nirholas/pump-fun-sdk
- Bags API docs: https://docs.bags.fm/how-to-guides/launch-token
- Bags SDK: `@bagsfm/bags-sdk` on npm
- Bags API key: https://dev.bags.fm/
