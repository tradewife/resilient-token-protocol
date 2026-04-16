export const RAW_IDL = 
{
  "address": "8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB",
  "metadata": {
    "name": "rtp_treasury",
    "version": "0.1.0",
    "spec": "0.1.0",
    "description": "Resilient Token Protocol — PDA-owned treasury with fee withdrawal, redistribution, swarm hydration, and phase evolution"
  },
  "instructions": [
    {
      "name": "check_redistribute",
      "docs": [
        "Check redistribution threshold and execute 70/20/10 split.",
        "",
        "Distributes the vault's excess above `min_runway_balance`:",
        "- 70% → holders",
        "- 20% → project dev wallet",
        "- 10% → ecosystem wallet (+ rounding dust)",
        "",
        "Callable by anyone. The split is deterministic on-chain."
      ],
      "discriminator": [
        47,
        16,
        230,
        59,
        244,
        188,
        14,
        159
      ],
      "accounts": [
        {
          "name": "mint",
          "docs": [
            "The Token-2022 mint."
          ]
        },
        {
          "name": "treasury",
          "docs": [
            "Treasury state account (PDA)."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  101,
                  97,
                  115,
                  117,
                  114,
                  121
                ]
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ]
          }
        },
        {
          "name": "treasury_vault",
          "docs": [
            "Treasury vault (source of redistribution).",
            "Authority = treasury PDA."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  101,
                  97,
                  115,
                  117,
                  114,
                  121
                ]
              },
              {
                "kind": "account",
                "path": "mint"
              },
              {
                "kind": "const",
                "value": [
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              }
            ]
          }
        },
        {
          "name": "holders_recipient",
          "docs": [
            "Holder distribution recipient token account.",
            "Authority verified against `treasury.holders_wallet`.",
            "Boxed to reduce stack frame size (BPF 4KB limit)."
          ],
          "writable": true
        },
        {
          "name": "dev_recipient",
          "docs": [
            "Project dev wallet token account. Authority verified against",
            "`treasury.project_dev_wallet` stored at initialization.",
            "Boxed to reduce stack frame size (BPF 4KB limit)."
          ],
          "writable": true
        },
        {
          "name": "ecosystem_recipient",
          "docs": [
            "Ecosystem wallet token account. Authority verified against",
            "`treasury.ecosystem_wallet` stored at initialization.",
            "Boxed to reduce stack frame size (BPF 4KB limit)."
          ],
          "writable": true
        },
        {
          "name": "token_program"
        }
      ],
      "args": []
    },
    {
      "name": "create_swarm_vault",
      "docs": [
        "Create the swarm hydration PDA vault.",
        "",
        "Must be called once after `initialize` and before `hydrate_swarm`.",
        "S-001 fix: replaces `init_if_needed` with explicit initialization",
        "to prevent re-initialization attacks on the swarm vault."
      ],
      "discriminator": [
        54,
        157,
        123,
        236,
        183,
        141,
        192,
        134
      ],
      "accounts": [
        {
          "name": "mint",
          "docs": [
            "The Token-2022 mint."
          ]
        },
        {
          "name": "treasury",
          "docs": [
            "Treasury state account (PDA)."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  101,
                  97,
                  115,
                  117,
                  114,
                  121
                ]
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ]
          }
        },
        {
          "name": "swarm_vault",
          "docs": [
            "Swarm hydration PDA vault. Created exactly once.",
            "Authority = treasury PDA."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  119,
                  97,
                  114,
                  109,
                  45,
                  104,
                  121,
                  100,
                  114,
                  97,
                  116,
                  105,
                  111,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ]
          }
        },
        {
          "name": "authority",
          "docs": [
            "Authority paying for vault creation (anyone can create)."
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "token_program"
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "evolve_phase",
      "docs": [
        "Evolve the treasury phase. IRREVERSIBLE.",
        "",
        "Phase thresholds (USDC value of treasury reserves):",
        "- Sustenance → Ecosystem:  >= $50k   (SUSTENANCE_CAP)",
        "- Ecosystem   → Humanity:  >= $1M    (ECOSYSTEM_CAP)",
        "",
        "Production: these thresholds should be validated against an on-chain",
        "oracle (e.g. Pyth). For devnet, the phase_authority signature is",
        "the guard — the authority is responsible for checking reserves.",
        "",
        "Only the treasury authority can trigger (Squads Multisig compatible)."
      ],
      "discriminator": [
        122,
        214,
        3,
        228,
        156,
        85,
        211,
        179
      ],
      "accounts": [
        {
          "name": "mint",
          "docs": [
            "The Token-2022 mint."
          ]
        },
        {
          "name": "treasury",
          "docs": [
            "Treasury state account (PDA)."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  101,
                  97,
                  115,
                  117,
                  114,
                  121
                ]
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ]
          }
        },
        {
          "name": "treasury_vault",
          "docs": [
            "Treasury vault — balance checked against phase caps (C-1 fix).",
            "Authority = treasury PDA."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  101,
                  97,
                  115,
                  117,
                  114,
                  121
                ]
              },
              {
                "kind": "account",
                "path": "mint"
              },
              {
                "kind": "const",
                "value": [
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              }
            ]
          }
        },
        {
          "name": "phase_authority",
          "docs": [
            "Phase authority — MUST be `treasury.authority`.",
            "Can be a Squads Multisig PDA for governance.",
            "S-002 fix: moved check here as Anchor constraint (single guard,",
            "spec-lock principle) — previously duplicated in handler body."
          ],
          "signer": true
        },
        {
          "name": "token_program"
        }
      ],
      "args": []
    },
    {
      "name": "force_retire_strategy",
      "docs": [
        "Emergency manual retirement by treasury authority. Bypasses thresholds."
      ],
      "discriminator": [
        109,
        185,
        175,
        217,
        122,
        36,
        219,
        63
      ],
      "accounts": [
        {
          "name": "treasury",
          "docs": [
            "Treasury state account (PDA, read-only, seeds verified)."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  101,
                  97,
                  115,
                  117,
                  114,
                  121
                ]
              },
              {
                "kind": "account",
                "path": "treasury.mint",
                "account": "Treasury"
              }
            ]
          }
        },
        {
          "name": "strategy_record",
          "docs": [
            "Strategy record PDA — mutable, seeds verified."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  116,
                  114,
                  97,
                  116,
                  101,
                  103,
                  121
                ]
              },
              {
                "kind": "account",
                "path": "treasury"
              },
              {
                "kind": "account",
                "path": "strategy_record.strategy_id",
                "account": "StrategyRecord"
              }
            ]
          }
        },
        {
          "name": "authority",
          "docs": [
            "Authority — must equal treasury.authority (enforced in handler)"
          ],
          "signer": true
        }
      ],
      "args": []
    },
    {
      "name": "hydrate_swarm",
      "docs": [
        "Fund swarm operations from the treasury vault.",
        "",
        "Enforces the 90-day runway invariant (CLAUDE.md #9):",
        "post-hydration balance MUST remain >= `min_runway_balance`.",
        "Transfers tokens to the swarm hydration PDA for swap to USDC."
      ],
      "discriminator": [
        212,
        99,
        122,
        236,
        170,
        17,
        27,
        28
      ],
      "accounts": [
        {
          "name": "mint",
          "docs": [
            "The Token-2022 mint."
          ]
        },
        {
          "name": "treasury",
          "docs": [
            "Treasury state account (PDA)."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  101,
                  97,
                  115,
                  117,
                  114,
                  121
                ]
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ]
          }
        },
        {
          "name": "treasury_vault",
          "docs": [
            "Treasury vault (hydration source).",
            "Authority = treasury PDA."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  101,
                  97,
                  115,
                  117,
                  114,
                  121
                ]
              },
              {
                "kind": "account",
                "path": "mint"
              },
              {
                "kind": "const",
                "value": [
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              }
            ]
          }
        },
        {
          "name": "swarm_vault",
          "docs": [
            "Swarm hydration PDA vault. Receives tokens for swap to USDC.",
            "Authority = treasury PDA. Must be explicitly initialized via",
            "`create_swarm_vault` before first hydration (S-001 fix:",
            "removed init_if_needed to prevent re-initialization attack)."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  119,
                  97,
                  114,
                  109,
                  45,
                  104,
                  121,
                  100,
                  114,
                  97,
                  116,
                  105,
                  111,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ]
          }
        },
        {
          "name": "strategy_record",
          "docs": [
            "Strategy record — MUST be Live to receive funding.",
            "Seeds: [STRATEGY_SEED, treasury.key(), strategy_id]"
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  116,
                  114,
                  97,
                  116,
                  101,
                  103,
                  121
                ]
              },
              {
                "kind": "account",
                "path": "treasury"
              },
              {
                "kind": "account",
                "path": "strategy_record.strategy_id",
                "account": "StrategyRecord"
              }
            ]
          }
        },
        {
          "name": "authority",
          "docs": [
            "Authority initiating hydration (anyone can trigger)."
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "token_program"
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "initialize",
      "docs": [
        "Initialize a new Treasury for a given Token-2022 mint.",
        "",
        "Prerequisites:",
        "- The mint MUST have TransferFeeConfig enabled with the Treasury PDA",
        "set as `withdraw_withheld_authority`. This is immutable once set.",
        "- The mint MUST be a Token-2022 mint.",
        "",
        "Called once per adopting token. Sets the PDA authority and vault."
      ],
      "discriminator": [
        175,
        175,
        109,
        31,
        13,
        152,
        155,
        237
      ],
      "accounts": [
        {
          "name": "mint",
          "docs": [
            "The Token-2022 mint adopting RTP.",
            "MUST have TransferFeeConfig enabled with the Treasury PDA as",
            "`withdraw_withheld_authority` (immutable once set)."
          ]
        },
        {
          "name": "treasury",
          "docs": [
            "Treasury state account (PDA). No private key exists."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  101,
                  97,
                  115,
                  117,
                  114,
                  121
                ]
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ]
          }
        },
        {
          "name": "treasury_vault",
          "docs": [
            "PDA-owned vault that receives withdrawn fees.",
            "Authority = treasury PDA (no human can sign for this)."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  101,
                  97,
                  115,
                  117,
                  114,
                  121
                ]
              },
              {
                "kind": "account",
                "path": "mint"
              },
              {
                "kind": "const",
                "value": [
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              }
            ]
          }
        },
        {
          "name": "holders_wallet",
          "docs": [
            "Holders wallet — receives 70% of redistribution.",
            "Stored as pubkey in treasury state for on-chain verification."
          ]
        },
        {
          "name": "project_dev_wallet",
          "docs": [
            "Project dev wallet — receives 20% of redistribution.",
            "Stored as pubkey in treasury state for on-chain verification."
          ]
        },
        {
          "name": "ecosystem_wallet",
          "docs": [
            "Ecosystem wallet — receives 10% of redistribution.",
            "Stored as pubkey in treasury state for on-chain verification."
          ]
        },
        {
          "name": "authority",
          "docs": [
            "Authority paying for initialization (anyone can initialize)."
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "token_program"
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "min_runway_balance",
          "type": "u64"
        }
      ]
    },
    {
      "name": "record_fee_deposit",
      "docs": [
        "Record a fee deposit from an adopting token project.",
        "",
        "Increments the AdopterRecord's cumulative fees and the treasury's",
        "total_fees_received_lamports. This is the accounting hook called",
        "alongside (or composed into) any fee deposit. It does not move",
        "funds — it only updates accounting state for pro-rata attribution."
      ],
      "discriminator": [
        82,
        195,
        165,
        45,
        104,
        133,
        190,
        185
      ],
      "accounts": [
        {
          "name": "adopter_record",
          "docs": [
            "AdopterRecord PDA — seeds: [\"adopter\", token_mint]"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  97,
                  100,
                  111,
                  112,
                  116,
                  101,
                  114
                ]
              },
              {
                "kind": "account",
                "path": "adopter_record.token_mint",
                "account": "AdopterRecord"
              }
            ]
          }
        },
        {
          "name": "treasury",
          "docs": [
            "Treasury state account — receives the total_fees_received_lamports increment"
          ],
          "writable": true
        },
        {
          "name": "authority",
          "docs": [
            "The authority that can record fee deposits"
          ],
          "signer": true
        }
      ],
      "args": [
        {
          "name": "amount_lamports",
          "type": "u64"
        }
      ]
    },
    {
      "name": "register_adopter",
      "docs": [
        "Register a new token project as an RTP adopter.",
        "",
        "Creates an AdopterRecord PDA for the given token mint. Called once",
        "per adopting token project at adoption time. The AdopterRecord tracks",
        "cumulative fee contributions for pro-rata yield attribution."
      ],
      "discriminator": [
        72,
        198,
        80,
        213,
        198,
        244,
        51,
        150
      ],
      "accounts": [
        {
          "name": "adopter_record",
          "docs": [
            "AdopterRecord PDA — one per token mint. Seeds: [\"adopter\", token_mint]"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  97,
                  100,
                  111,
                  112,
                  116,
                  101,
                  114
                ]
              },
              {
                "kind": "arg",
                "path": "token_mint"
              }
            ]
          }
        },
        {
          "name": "treasury",
          "docs": [
            "The treasury state account (must already be initialised)"
          ],
          "writable": true
        },
        {
          "name": "authority",
          "docs": [
            "The authority signing this registration"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "token_mint",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "register_strategy",
      "docs": [
        "Register (promote) a strategy from the Python research layer into",
        "on-chain LIVE status. Only callable by `treasury.authority`."
      ],
      "discriminator": [
        121,
        12,
        64,
        75,
        99,
        15,
        177,
        143
      ],
      "accounts": [
        {
          "name": "treasury",
          "docs": [
            "Treasury state account (PDA, read-only)."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  101,
                  97,
                  115,
                  117,
                  114,
                  121
                ]
              },
              {
                "kind": "account",
                "path": "treasury.mint",
                "account": "Treasury"
              }
            ]
          }
        },
        {
          "name": "strategy_record",
          "docs": [
            "Strategy record PDA — init, seeds: [STRATEGY_SEED, treasury, strategy_id]"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  116,
                  114,
                  97,
                  116,
                  101,
                  103,
                  121
                ]
              },
              {
                "kind": "account",
                "path": "treasury"
              },
              {
                "kind": "arg",
                "path": "strategy_id"
              }
            ]
          }
        },
        {
          "name": "authority",
          "docs": [
            "Authority — must equal treasury.authority"
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "system_program",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "strategy_id",
          "type": "string"
        },
        {
          "name": "promotion_sharpe_x100",
          "type": "i32"
        }
      ]
    },
    {
      "name": "update_strategy_performance",
      "docs": [
        "Update strategy performance metrics after each completed trade batch.",
        "Enforces hard stop and soft decay thresholds automatically."
      ],
      "discriminator": [
        235,
        188,
        63,
        220,
        30,
        125,
        240,
        85
      ],
      "accounts": [
        {
          "name": "treasury",
          "docs": [
            "Treasury state account (PDA, read-only)."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  101,
                  97,
                  115,
                  117,
                  114,
                  121
                ]
              },
              {
                "kind": "account",
                "path": "treasury.mint",
                "account": "Treasury"
              }
            ]
          }
        },
        {
          "name": "strategy_record",
          "docs": [
            "Strategy record PDA — mutable, seeds verified."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  116,
                  114,
                  97,
                  116,
                  101,
                  103,
                  121
                ]
              },
              {
                "kind": "account",
                "path": "treasury"
              },
              {
                "kind": "account",
                "path": "strategy_record.strategy_id",
                "account": "StrategyRecord"
              }
            ]
          }
        },
        {
          "name": "authority",
          "docs": [
            "Authority — must equal treasury.authority"
          ],
          "signer": true
        }
      ],
      "args": [
        {
          "name": "rolling_pnl_bps",
          "type": "i32"
        },
        {
          "name": "rolling_sharpe_x100",
          "type": "i32"
        },
        {
          "name": "consecutive_losses",
          "type": "u8"
        },
        {
          "name": "drawdown_24h_bps",
          "type": "u16"
        },
        {
          "name": "new_soft_strike",
          "type": "bool"
        }
      ]
    },
    {
      "name": "verify_adoption",
      "docs": [
        "Verify that the mint has TransferFeeConfig enabled and that the",
        "Treasury PDA is the `withdraw_withheld_authority`.",
        "",
        "READ-ONLY instruction — no state mutation. Deserializes the mint",
        "account data (base Mint + TLV extensions) and confirms the withdraw",
        "authority matches the Treasury PDA.",
        "",
        "SL-001/SL-002 fix: on-chain adoption verification instead of",
        "relying on off-chain \"did you configure the mint?\" trust."
      ],
      "discriminator": [
        0,
        226,
        116,
        244,
        196,
        245,
        96,
        151
      ],
      "accounts": [
        {
          "name": "mint",
          "docs": [
            "The Token-2022 mint — MUST have TransferFeeConfig enabled."
          ]
        },
        {
          "name": "treasury",
          "docs": [
            "Treasury state account (PDA)."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  101,
                  97,
                  115,
                  117,
                  114,
                  121
                ]
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ]
          }
        },
        {
          "name": "token_program"
        }
      ],
      "args": []
    },
    {
      "name": "withdraw_fees",
      "docs": [
        "Withdraw accumulated TransferFeeConfig fees from mint into treasury vault.",
        "",
        "Uses CPI: `spl_token_2022::withdraw_withheld_tokens_from_mint`",
        "The Treasury PDA (set as `withdraw_withheld_authority` on the mint at",
        "adoption time) signs for the withdrawal. Anyone can call this — fees",
        "are permissionlessly pulled into the PDA."
      ],
      "discriminator": [
        198,
        212,
        171,
        109,
        144,
        215,
        174,
        89
      ],
      "accounts": [
        {
          "name": "mint",
          "docs": [
            "The Token-2022 mint with TransferFeeConfig enabled.",
            "`mut` required: CPI `withdraw_withheld_tokens_from_mint` marks",
            "mint as writable in its account metas."
          ],
          "writable": true
        },
        {
          "name": "treasury",
          "docs": [
            "Treasury state account (PDA)."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  101,
                  97,
                  115,
                  117,
                  114,
                  121
                ]
              },
              {
                "kind": "account",
                "path": "mint"
              }
            ]
          }
        },
        {
          "name": "treasury_vault",
          "docs": [
            "Treasury vault where withdrawn fees land.",
            "Authority = treasury PDA."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  101,
                  97,
                  115,
                  117,
                  114,
                  121
                ]
              },
              {
                "kind": "account",
                "path": "mint"
              },
              {
                "kind": "const",
                "value": [
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              }
            ]
          }
        },
        {
          "name": "token_program"
        }
      ],
      "args": []
    }
  ],
  "accounts": [
    {
      "name": "AdopterRecord",
      "discriminator": [
        24,
        49,
        180,
        138,
        124,
        127,
        2,
        189
      ]
    },
    {
      "name": "StrategyRecord",
      "discriminator": [
        69,
        98,
        219,
        95,
        17,
        64,
        37,
        17
      ]
    },
    {
      "name": "Treasury",
      "discriminator": [
        238,
        239,
        123,
        238,
        89,
        1,
        168,
        253
      ]
    }
  ],
  "events": [
    {
      "name": "AdopterRegistered",
      "discriminator": [
        215,
        53,
        71,
        4,
        59,
        62,
        207,
        4
      ]
    },
    {
      "name": "FeeDepositRecorded",
      "discriminator": [
        140,
        20,
        33,
        182,
        145,
        255,
        131,
        240
      ]
    },
    {
      "name": "StrategyPerformanceUpdated",
      "discriminator": [
        8,
        136,
        235,
        70,
        79,
        83,
        170,
        186
      ]
    },
    {
      "name": "StrategyPromoted",
      "discriminator": [
        86,
        248,
        82,
        97,
        48,
        235,
        105,
        210
      ]
    },
    {
      "name": "StrategyRetired",
      "discriminator": [
        175,
        215,
        69,
        207,
        91,
        79,
        120,
        152
      ]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "BelowThreshold",
      "msg": "Treasury reserves below redistribution threshold"
    },
    {
      "code": 6001,
      "name": "InsufficientRunway",
      "msg": "Post-hydration balance would fall below the 90-day runway minimum"
    },
    {
      "code": 6002,
      "name": "HydrationExceedsBalance",
      "msg": "Hydration amount exceeds available balance"
    },
    {
      "code": 6003,
      "name": "AlreadyMaxPhase",
      "msg": "Treasury is already at maximum phase (Humanity)"
    },
    {
      "code": 6004,
      "name": "UnauthorizedPhaseEvolution",
      "msg": "Only the treasury authority can evolve phases"
    },
    {
      "code": 6005,
      "name": "WithdrawAuthorityMismatch",
      "msg": "Mint's withdraw_withheld_authority does not match Treasury PDA"
    },
    {
      "code": 6006,
      "name": "MintNotConfigured",
      "msg": "Mint does not have TransferFeeConfig enabled — cannot adopt RTP"
    },
    {
      "code": 6007,
      "name": "ZeroAmount",
      "msg": "Fee deposit amount must be greater than zero"
    },
    {
      "code": 6008,
      "name": "Overflow",
      "msg": "Arithmetic overflow in fee accounting"
    },
    {
      "code": 6009,
      "name": "StrategyNotLive",
      "msg": "Strategy is not in Live status — cannot fund or trade"
    },
    {
      "code": 6010,
      "name": "HardStopBreached",
      "msg": "Strategy has breached a hard stop threshold"
    },
    {
      "code": 6011,
      "name": "SoftDecayRetirement",
      "msg": "Strategy has accumulated too many soft decay strikes"
    },
    {
      "code": 6012,
      "name": "InvalidStrategyId",
      "msg": "Strategy ID must be 1–16 characters"
    },
    {
      "code": 6013,
      "name": "UnauthorizedStrategyOp",
      "msg": "Only the treasury authority can register or retire strategies"
    }
  ],
  "types": [
    {
      "name": "AdopterRecord",
      "docs": [
        "Tracks a single token project's cumulative fee contributions to the RTP treasury.",
        "One AdopterRecord PDA per adopting token mint.",
        "Seeds: [\"adopter\", token_mint.key()]",
        "This enables pro-rata yield attribution:",
        "adopter_yield_share = fees_contributed_lamports / treasury.total_fees_received_lamports"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "token_mint",
            "docs": [
              "The SPL token mint of the adopting project"
            ],
            "type": "pubkey"
          },
          {
            "name": "fees_contributed_lamports",
            "docs": [
              "Cumulative fee contributions (in lamports) since adoption"
            ],
            "type": "u64"
          },
          {
            "name": "adopted_at",
            "docs": [
              "Unix timestamp of first fee deposit (adoption date)"
            ],
            "type": "i64"
          },
          {
            "name": "last_deposit_ts",
            "docs": [
              "Unix timestamp of most recent fee deposit"
            ],
            "type": "i64"
          },
          {
            "name": "deposit_count",
            "docs": [
              "Number of discrete fee deposits recorded"
            ],
            "type": "u64"
          },
          {
            "name": "bump",
            "docs": [
              "PDA bump"
            ],
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "AdopterRegistered",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "token_mint",
            "type": "pubkey"
          },
          {
            "name": "adopted_at",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "FeeDepositRecorded",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "token_mint",
            "type": "pubkey"
          },
          {
            "name": "amount_lamports",
            "type": "u64"
          },
          {
            "name": "cumulative",
            "type": "u64"
          },
          {
            "name": "total_treasury_fees",
            "type": "u64"
          },
          {
            "name": "ts",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "Phase",
      "docs": [
        "Treasury phase — can only advance forward. Transitions are IRREVERSIBLE.",
        "- Sustenance (<$50k): self-hydrate, reinvest all yield",
        "- Ecosystem ($50k-$1M): auto-provide LP to top RTP-adopting tokens",
        "- Humanity (>$1M): USDC grants to Solana public-goods projects"
      ],
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Sustenance"
          },
          {
            "name": "Ecosystem"
          },
          {
            "name": "Humanity"
          }
        ]
      }
    },
    {
      "name": "RetirementReason",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "HardDrawdown"
          },
          {
            "name": "ConsecutiveLosses"
          },
          {
            "name": "RollingSharpeLow"
          },
          {
            "name": "SoftDecayStrikes"
          },
          {
            "name": "AuthorityForced"
          }
        ]
      }
    },
    {
      "name": "StrategyLifecycleStatus",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "Live"
          },
          {
            "name": "Suspended"
          },
          {
            "name": "Retired"
          }
        ]
      }
    },
    {
      "name": "StrategyPerformanceUpdated",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "treasury",
            "type": "pubkey"
          },
          {
            "name": "strategy_id",
            "type": "string"
          },
          {
            "name": "rolling_pnl_bps",
            "type": "i32"
          },
          {
            "name": "rolling_sharpe_x100",
            "type": "i32"
          },
          {
            "name": "consecutive_losses",
            "type": "u8"
          },
          {
            "name": "soft_decay_strikes",
            "type": "u8"
          },
          {
            "name": "drawdown_24h_bps",
            "type": "u16"
          },
          {
            "name": "status",
            "type": {
              "defined": {
                "name": "StrategyLifecycleStatus"
              }
            }
          },
          {
            "name": "ts",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "StrategyPromoted",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "treasury",
            "type": "pubkey"
          },
          {
            "name": "strategy_id",
            "type": "string"
          },
          {
            "name": "promotion_sharpe_x100",
            "type": "i32"
          },
          {
            "name": "promoted_at",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "StrategyRecord",
      "docs": [
        "On-chain lifecycle ledger for a single trading strategy.",
        "Seeds: [STRATEGY_SEED, treasury.key(), strategy_id.as_bytes()]"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "treasury",
            "docs": [
              "The treasury this strategy belongs to"
            ],
            "type": "pubkey"
          },
          {
            "name": "strategy_id",
            "docs": [
              "Unique strategy identifier (max 16 bytes, e.g. \"S03\", \"SOL_CARRY_v1\")"
            ],
            "type": "string"
          },
          {
            "name": "status",
            "docs": [
              "Current lifecycle status"
            ],
            "type": {
              "defined": {
                "name": "StrategyLifecycleStatus"
              }
            }
          },
          {
            "name": "promoted_at",
            "docs": [
              "Unix timestamp when strategy was promoted to LIVE"
            ],
            "type": "i64"
          },
          {
            "name": "last_update_ts",
            "docs": [
              "Unix timestamp of last performance update"
            ],
            "type": "i64"
          },
          {
            "name": "rolling_pnl_bps",
            "docs": [
              "Rolling 30-day PnL in basis points (signed, scaled x100)",
              "e.g. +350 = +3.50%, -120 = -1.20%"
            ],
            "type": "i32"
          },
          {
            "name": "consecutive_losses",
            "docs": [
              "Number of consecutive losing trades (reset on any win)"
            ],
            "type": "u8"
          },
          {
            "name": "soft_decay_strikes",
            "docs": [
              "Number of soft decay strikes accumulated"
            ],
            "type": "u8"
          },
          {
            "name": "drawdown_24h_bps",
            "docs": [
              "Largest single drawdown observed in the last 24h, in basis points"
            ],
            "type": "u16"
          },
          {
            "name": "total_trades",
            "docs": [
              "Cumulative total trades executed on-chain"
            ],
            "type": "u32"
          },
          {
            "name": "promotion_sharpe_x100",
            "docs": [
              "Sharpe ratio at time of promotion (stored as integer x100, e.g. 396 = 3.96)"
            ],
            "type": "i32"
          },
          {
            "name": "rolling_sharpe_x100",
            "docs": [
              "Current rolling Sharpe (integer x100). Updated by the swarm agent."
            ],
            "type": "i32"
          },
          {
            "name": "bump",
            "docs": [
              "PDA bump"
            ],
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "StrategyRetired",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "treasury",
            "type": "pubkey"
          },
          {
            "name": "strategy_id",
            "type": "string"
          },
          {
            "name": "reason",
            "type": {
              "defined": {
                "name": "RetirementReason"
              }
            }
          },
          {
            "name": "final_rolling_sharpe_x100",
            "type": "i32"
          },
          {
            "name": "ts",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "Treasury",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "docs": [
              "The Token-2022 mint this treasury serves"
            ],
            "type": "pubkey"
          },
          {
            "name": "authority",
            "docs": [
              "The phase authority (set at initialization)."
            ],
            "type": "pubkey"
          },
          {
            "name": "phase",
            "docs": [
              "Current evolution phase (Sustenance → Ecosystem → Humanity)"
            ],
            "type": {
              "defined": {
                "name": "Phase"
              }
            }
          },
          {
            "name": "total_fees_withdrawn",
            "docs": [
              "Cumulative fees withdrawn from mint via TransferFeeConfig"
            ],
            "type": "u64"
          },
          {
            "name": "total_distributed_holders",
            "docs": [
              "Cumulative tokens distributed to holders (70%)"
            ],
            "type": "u64"
          },
          {
            "name": "total_distributed_dev",
            "docs": [
              "Cumulative tokens distributed to project dev (20%)"
            ],
            "type": "u64"
          },
          {
            "name": "total_distributed_ecosystem",
            "docs": [
              "Cumulative tokens distributed to ecosystem (10%)"
            ],
            "type": "u64"
          },
          {
            "name": "total_hydration",
            "docs": [
              "Cumulative tokens sent to swarm hydration vault"
            ],
            "type": "u64"
          },
          {
            "name": "total_fees_received_lamports",
            "docs": [
              "Cumulative fee contributions recorded from all adopters via record_fee_deposit.",
              "Denominator for pro-rata yield attribution:",
              "adopter_yield_share = fees_contributed / total_fees_received_lamports * yield_pool"
            ],
            "type": "u64"
          },
          {
            "name": "holders_wallet",
            "docs": [
              "Holders wallet (receives 70% of redistribution)"
            ],
            "type": "pubkey"
          },
          {
            "name": "project_dev_wallet",
            "docs": [
              "Project dev wallet (receives 20% of redistribution)"
            ],
            "type": "pubkey"
          },
          {
            "name": "ecosystem_wallet",
            "docs": [
              "Ecosystem wallet (receives 10% of redistribution)"
            ],
            "type": "pubkey"
          },
          {
            "name": "min_runway_balance",
            "docs": [
              "Minimum balance that must remain after hydration.",
              "Enforces the 90-day runway invariant (CLAUDE.md #9).",
              "Production: set to USDC-denominated 90-day ops cost via oracle."
            ],
            "type": "u64"
          },
          {
            "name": "bump",
            "docs": [
              "PDA bump"
            ],
            "type": "u8"
          }
        ]
      }
    }
  ]
};
