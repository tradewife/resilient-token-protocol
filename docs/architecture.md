# RTP Architecture

## Three-Layer Stack

```mermaid
graph TD
    subgraph RESEARCH["Python Research Layer"]
        NS["Night Shift<br/>30K configs/night<br/>9-fold WFA"]
        PT["Paper Trader<br/>Live Binance data<br/>State: data/paper_trading/"]
        BS["Bridge Binary<br/>night_shift.bin<br/>Typed JSON interface"]
    end

    subgraph SWARM["Rust Swarm Runtime"]
        COORD["Coordinator<br/>Soulguard → Router → Audit"]
        TW["Trading Wing<br/>HL execution<br/>EIP-712 signing"]
        SW["Security Wing<br/>Threat detection"]
        EW["Evolve Wing<br/>LLM mutations<br/>Strategy adaptation"]
        KW["Knowledge Wing<br/>Cross-wing memory"]
        AW["Audit Wing<br/>3-agent tribunal<br/>Byzantine consensus"]
        FW["Futureproof Wing<br/>Deprecation monitoring"]
    end

    subgraph ONCHAIN["Solana On-Chain (Anchor)"]
        TP["Treasury PDA<br/>FNQbK1...otF"]
        RG["Redistribute<br/>70/20/10 split"]
        HY["Self-Hydrate<br/>90-day runway check"]
        PH["Phase Evolution<br/>Sustenance → Ecosystem → Humanity"]
    end

    subgraph EXEC["Hyperliquid (Perps Execution)"]
        HL["HL Testnet API<br/>REST + EIP-712"]
        YD["Yield (USDC)<br/>PnL → Treasury deposit"]
    end

    NS --> BS
    BS --> TW
    TW --> COORD
    COORD --> AW
    COORD --> TW
    EW --> COORD
    TW --> HL
    HL --> YD
    YD --> TP
    TP --> RG
    TP --> HY
    TP --> PH
```

## Data Flow

```
[Python Night Shift]                    [Rust Swarm]                    [Solana Devnet]
     │                                       │                                │
     ├── 30K config grid search              │                                │
     ├── 9-fold walk-forward analysis        │                                │
     ├── Darwinian mutations                 │                                │
     │                                       │                                │
     └── bridge.rs ──typed JSON──►  Trading Wing                           │
                                       │                                    │
                                  Soulguard check                          │
                                       │                                    │
                                  Audit tribunal                           │
                                  (Skeptic/UserProxy/Optimizer)            │
                                       │                                    │
                                  ExecutePermit ──►  Hyperliquid POST      │
                                       │            (EIP-712 signed)       │
                                       │                    │               │
                                       │              Fill confirmed        │
                                       │                    │               │
                                       │              YieldReport ◄────────┘
                                       │                    │
                                  Memory persist       Treasury CPI transfer
                                  (data/swarm-memory/)  (USDC yield → SOL → PDA)
```

## Capital Flow (Unified SOL Cycle)

```
SOL in (fees) → Phantom bridge → USDC → HL perps → USDC yield → Phantom bridge → SOL → Treasury PDA
                 (mainnet)        ↑                    ↓           (mainnet)
                              HL clearinghouse    USDC-margined
                              holds working cap   perps positions

Devnet: HL funded directly via faucet. devnet_fund_stub() simulates bridge for demo.
```

## Autonomous Devnet Loop

```
GitHub Actions (cron: every 6h)
  │
  ├── cargo run --bin rtp-daemon
  │     │
  │     ├── Load config from data/devnet-cycles/latest/config.json
  │     ├── Run orchestrator cycle (healthy/declining/stagnant states)
  │     ├── Propose strategy mutations (LLM or deterministic fallback)
  │     ├── Validate against soulcontract bounds
  │     ├── Persist memory to data/swarm-memory/
  │     └── Write cycle output to data/devnet-cycles/YYYY-MM-DDTHH/
  │
  ├── git add data/devnet-cycles/
  └── git commit + push
```
