export const RAW_IDL = 
{
  "address": "8rt6yiBnRTyHy8F69jUd7exWwwShUs4Eokeq41auo2RB",
  "metadata": {
    "name": "rtp_treasury",
    "version": "0.1.0",
    "spec": "0.1.0",
    "description": "Resilient Token Protocol \u2014 PDA-owned treasury with fee withdrawal, redistribution, swarm hydration, and phase evolution"
  },
  "instructions": [
    {
      "name": "check_redistribute",
      "docs": [
        "Check redistribution threshold and execute 70/20/10 split.",
        "",
        "Distributes the vault's excess above `min_runway_balance`:",
        "- 70% \u2192 holders",
        "- 20% \u2192 project dev wallet",
        "- 10% \u2192 ecosystem wallet (+ rounding dust)",
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
      "name": "close_flash_position",
      "docs": [
        "Close a Flash Trade perpetual position via CPI.",
        "",
        "Closing is permitted even if strategy is Suspended (exiting is always safe).",
        "Treasury frozen check still applies.",
        "",
        "Flash Trade close_position accounts (18 accounts from IDL v15.2.0):",
        "0: owner (treasury PDA, signer)",
        "1: fee_payer (authority)",
        "2: receiving_account (writable)",
        "3: transfer_authority",
        "4: perpetuals",
        "5: pool (writable)",
        "6: position (writable)",
        "7: market (writable)",
        "8: target_custody",
        "9: target_oracle_account",
        "10: collateral_custody (writable)",
        "11: collateral_oracle_account",
        "12: collateral_custody_token_account (writable)",
        "13: token_program",
        "14: event_authority",
        "15: program",
        "16: ix_sysvar",
        "17: collateral_mint"
      ],
      "discriminator": [
        65,
        15,
        74,
        221,
        107,
        136,
        176,
        33
      ],
      "accounts": [
        {
          "name": "treasury",
          "docs": [
            "Treasury state account (PDA, mutable for event emission)."
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
                "path": "treasury.mint",
                "account": "Treasury"
              }
            ]
          }
        },
        {
          "name": "strategy_record",
          "docs": [
            "Strategy record \u2014 mutable for position count update.",
            "Close is permitted even if Suspended (exiting is always safe)."
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
            "Fee payer."
          ],
          "writable": true,
          "signer": true
        }
      ],
      "args": [
        {
          "name": "side",
          "type": {
            "defined": {
              "name": "FlashSide"
            }
          }
        },
        {
          "name": "oracle_price",
          "type": {
            "defined": {
              "name": "FlashOraclePrice"
            }
          }
        },
        {
          "name": "slippage_bps",
          "type": "u16"
        },
        {
          "name": "committed_sol_lamports_delta",
          "type": "u64"
        }
      ]
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
      "name": "emergency_close_all_positions",
      "docs": [
        "Emergency reset of Flash Trade position counters.",
        "Authority-gated. Designed to be called *together with* `freeze_treasury`",
        "(in either order) so it is intentionally NOT blocked by `treasury.frozen`.",
        "",
        "What it does:",
        "1. Resets `open_position_count` and `committed_sol_lamports` to 0",
        "2. Emits an `EmergencyPositionsReset` event for the audit trail",
        "",
        "What it does NOT do:",
        "- It does **not** invoke Flash Trade CPI close. Operators must follow",
        "up with explicit `close_flash_position` calls per position (or rely",
        "on Flash Trade keeper liquidation) to actually unwind exposure.",
        "- The event is deliberately distinct from `FlashPositionClosed` so",
        "observers cannot mistake a counter reset for a real position close."
      ],
      "discriminator": [
        7,
        159,
        254,
        118,
        225,
        67,
        115,
        184
      ],
      "accounts": [
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
                "path": "treasury.mint",
                "account": "Treasury"
              }
            ]
          }
        },
        {
          "name": "strategy_record",
          "docs": [
            "Strategy record \u2014 counters reset to zero."
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
            "Authority \u2014 must equal treasury.authority."
          ],
          "signer": true
        }
      ],
      "args": [
        {
          "name": "position_pubkeys",
          "type": {
            "vec": "pubkey"
          }
        }
      ]
    },
    {
      "name": "end_beta",
      "docs": [
        "End a beta adopter's RTP participation early.",
        "",
        "Only callable by `treasury.authority`. Sets `beta_ended = true`,",
        "which prevents further hydrate_swarm funding for this adopter.",
        "The adopter's fee contributions remain on record for attribution.",
        "Yield already generated stays with the project."
      ],
      "discriminator": [
        7,
        114,
        33,
        172,
        76,
        192,
        47,
        49
      ],
      "accounts": [
        {
          "name": "adopter_record",
          "docs": [
            "AdopterRecord PDA \u2014 seeds: [\"adopter\", token_mint]"
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
          "name": "authority",
          "docs": [
            "Authority \u2014 must equal treasury.authority (enforced in handler)"
          ],
          "signer": true
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
        "- Sustenance \u2192 Ecosystem:  >= $50k   (SUSTENANCE_CAP)",
        "- Ecosystem   \u2192 Humanity:  >= $1M    (ECOSYSTEM_CAP)",
        "",
        "Production: these thresholds should be validated against an on-chain",
        "oracle (e.g. Pyth). For devnet, the phase_authority signature is",
        "the guard \u2014 the authority is responsible for checking reserves.",
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
            "Treasury vault \u2014 balance checked against phase caps (C-1 fix).",
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
            "Phase authority \u2014 MUST be `treasury.authority`.",
            "Can be a Squads Multisig PDA for governance.",
            "S-002 fix: moved check here as Anchor constraint (single guard,",
            "spec-lock principle) \u2014 previously duplicated in handler body."
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
            "Strategy record PDA \u2014 mutable, seeds verified."
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
            "Authority \u2014 must equal treasury.authority (enforced in handler)"
          ],
          "signer": true
        }
      ],
      "args": []
    },
    {
      "name": "freeze_treasury",
      "docs": [
        "Emergency freeze: authority-gated, sets frozen = true.",
        "In production, authority is the Squads multisig PDA \u2014 requires 2-of-3 approval.",
        "No time lock on freeze (emergency speed). Unfreeze requires 24h time lock."
      ],
      "discriminator": [
        11,
        162,
        24,
        48,
        89,
        121,
        169,
        188
      ],
      "accounts": [
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
                "path": "treasury.mint",
                "account": "Treasury"
              }
            ]
          }
        },
        {
          "name": "authority",
          "docs": [
            "Authority \u2014 must equal treasury.authority."
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
            "Strategy record \u2014 MUST be Live to receive funding.",
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
          "name": "adopter_record",
          "docs": [
            "Adopter record for beta expiry check. Seeds: [\"adopter\", token_mint]",
            "If beta_expires_at > 0 and the beta has expired or been ended,",
            "hydrate_swarm is refused."
          ],
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
            "Holders wallet \u2014 receives 70% of redistribution.",
            "Stored as pubkey in treasury state for on-chain verification."
          ]
        },
        {
          "name": "project_dev_wallet",
          "docs": [
            "Project dev wallet \u2014 receives 20% of redistribution.",
            "Stored as pubkey in treasury state for on-chain verification."
          ]
        },
        {
          "name": "ecosystem_wallet",
          "docs": [
            "Ecosystem wallet \u2014 receives 10% of redistribution.",
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
      "name": "open_flash_position",
      "docs": [
        "Open a Flash Trade perpetual position via CPI, signed by Treasury PDA.",
        "",
        "Constraints enforced before CPI:",
        "1. Treasury not frozen",
        "2. Strategy must be Live",
        "3. open_position_count < MAX_CONCURRENT_POSITIONS (3)",
        "4. Vault balance after commit >= min_runway_balance",
        "5. input_sol_lamports <= vault * MAX_POSITION_SIZE_BPS / 10000",
        "",
        "Flash Trade accounts are passed via remaining_accounts in IDL v15.2.0 order:",
        "0: owner (treasury PDA, signer via invoke_signed)",
        "1: fee_payer (authority, pays rent)",
        "2: funding_account (WSOL temp account)",
        "3: transfer_authority (Flash Trade PDA)",
        "4: perpetuals (Flash Trade PDA)",
        "5: pool (writable)",
        "6: position (writable, PDA)",
        "7: market (writable)",
        "8: target_custody",
        "9: target_oracle_account",
        "10: collateral_custody (writable)",
        "11: collateral_oracle_account",
        "12: collateral_custody_token_account (writable)",
        "13: system_program",
        "14: funding_token_program",
        "15: event_authority (Flash Trade PDA)",
        "16: program (Flash Trade program ID)",
        "17: ix_sysvar",
        "18: funding_mint"
      ],
      "discriminator": [
        102,
        68,
        197,
        231,
        254,
        69,
        188,
        127
      ],
      "accounts": [
        {
          "name": "treasury",
          "docs": [
            "Treasury state account (PDA, mutable for event emission)."
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
                "path": "treasury.mint",
                "account": "Treasury"
              }
            ]
          }
        },
        {
          "name": "strategy_record",
          "docs": [
            "Strategy record \u2014 must be Live to open positions."
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
          "name": "treasury_vault",
          "docs": [
            "Treasury vault \u2014 Token-2022 token account whose token amount denominates",
            "`min_runway_balance` and `input_sol_lamports`. Authority = treasury PDA.",
            "Seeds verified; runway/position-size checks read `.amount`, not `.lamports()`."
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
                "path": "treasury.mint",
                "account": "Treasury"
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
          "name": "authority",
          "docs": [
            "Fee payer \u2014 pays for transaction gas and Flash Trade account rent.",
            "Has NO authority over treasury funds (only pays gas)."
          ],
          "writable": true,
          "signer": true
        }
      ],
      "args": [
        {
          "name": "side",
          "type": {
            "defined": {
              "name": "FlashSide"
            }
          }
        },
        {
          "name": "input_sol_lamports",
          "type": "u64"
        },
        {
          "name": "leverage_bps",
          "type": "u32"
        },
        {
          "name": "slippage_bps",
          "type": "u16"
        },
        {
          "name": "oracle_price",
          "type": {
            "defined": {
              "name": "FlashOraclePrice"
            }
          }
        },
        {
          "name": "pool_name",
          "type": "string"
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
        "funds \u2014 it only updates accounting state for pro-rata attribution."
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
            "AdopterRecord PDA \u2014 seeds: [\"adopter\", token_mint]"
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
            "Treasury state account \u2014 receives the total_fees_received_lamports increment"
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
        "Register a new token project as an RTP adopter (permanent \u2014 no expiry).",
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
            "AdopterRecord PDA \u2014 one per token mint. Seeds: [\"adopter\", token_mint]"
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
      "name": "register_adopter_beta",
      "docs": [
        "Register a beta adopter with an automatic expiry timestamp.",
        "",
        "Same as register_adopter but sets `beta_expires_at`. After this",
        "timestamp, hydrate_swarm will refuse to fund strategies for this",
        "adopter. The beta can also be ended early via `end_beta`.",
        "",
        "Typical use: Colosseum hackathon beta \u2014 expires 1 week after the",
        "hackathon deadline."
      ],
      "discriminator": [
        108,
        155,
        116,
        10,
        100,
        217,
        144,
        173
      ],
      "accounts": [
        {
          "name": "adopter_record",
          "docs": [
            "AdopterRecord PDA \u2014 one per token mint. Seeds: [\"adopter\", token_mint]"
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
        },
        {
          "name": "beta_expires_at",
          "type": "i64"
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
            "Strategy record PDA \u2014 init, seeds: [STRATEGY_SEED, treasury, strategy_id]"
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
            "Authority \u2014 must equal treasury.authority"
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
      "name": "unfreeze_treasury",
      "docs": [
        "Unfreeze: authority-gated, sets frozen = false.",
        "In production, requires Squads 2-of-3 + 24h time lock."
      ],
      "discriminator": [
        71,
        1,
        11,
        192,
        79,
        138,
        250,
        129
      ],
      "accounts": [
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
                "path": "treasury.mint",
                "account": "Treasury"
              }
            ]
          }
        },
        {
          "name": "authority",
          "docs": [
            "Authority \u2014 must equal treasury.authority."
          ],
          "signer": true
        }
      ],
      "args": []
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
            "Strategy record PDA \u2014 mutable, seeds verified."
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
            "Authority \u2014 must equal treasury.authority"
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
        "READ-ONLY instruction \u2014 no state mutation. Deserializes the mint",
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
            "The Token-2022 mint \u2014 MUST have TransferFeeConfig enabled."
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
        "adoption time) signs for the withdrawal. Anyone can call this \u2014 fees",
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
      "name": "BetaEnded",
      "discriminator": [
        63,
        18,
        196,
        45,
        156,
        43,
        24,
        201
      ]
    },
    {
      "name": "EmergencyPositionsReset",
      "discriminator": [
        5,
        226,
        219,
        166,
        101,
        144,
        16,
        102
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
      "name": "FlashPositionClosed",
      "discriminator": [
        203,
        247,
        76,
        93,
        1,
        35,
        157,
        137
      ]
    },
    {
      "name": "FlashPositionOpened",
      "discriminator": [
        250,
        242,
        203,
        152,
        87,
        88,
        251,
        57
      ]
    },
    {
      "name": "Redistribution",
      "discriminator": [
        233,
        133,
        152,
        65,
        89,
        103,
        79,
        145
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
    },
    {
      "name": "TreasuryFrozen",
      "discriminator": [
        93,
        25,
        3,
        194,
        186,
        48,
        201,
        185
      ]
    },
    {
      "name": "TreasuryUnfrozen",
      "discriminator": [
        178,
        174,
        48,
        234,
        92,
        48,
        128,
        47
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
      "msg": "Mint does not have TransferFeeConfig enabled \u2014 cannot adopt RTP"
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
      "msg": "Strategy is not in Live status \u2014 cannot fund or trade"
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
      "msg": "Strategy ID must be 1\u201316 characters"
    },
    {
      "code": 6013,
      "name": "UnauthorizedStrategyOp",
      "msg": "Only the treasury authority can register or retire strategies"
    },
    {
      "code": 6014,
      "name": "BetaExpired",
      "msg": "Beta period has expired \u2014 operations no longer permitted"
    },
    {
      "code": 6015,
      "name": "UnauthorizedBetaOp",
      "msg": "Only the treasury authority can end a beta"
    },
    {
      "code": 6016,
      "name": "ZeroAddressRejected",
      "msg": "Zero address (Pubkey::default()) is not allowed"
    },
    {
      "code": 6017,
      "name": "TreasuryFrozen",
      "msg": "Treasury is frozen \u2014 all operations are halted"
    },
    {
      "code": 6018,
      "name": "AlreadyFrozen",
      "msg": "Treasury is already frozen"
    },
    {
      "code": 6019,
      "name": "NotFrozen",
      "msg": "Treasury is not frozen"
    },
    {
      "code": 6020,
      "name": "TooManyOpenPositions",
      "msg": "Too many concurrent Flash Trade positions (max 3)"
    },
    {
      "code": 6021,
      "name": "PositionSizeExceeded",
      "msg": "Input SOL exceeds maximum position size (20% of vault)"
    },
    {
      "code": 6022,
      "name": "PositionNotOwnedByTreasury",
      "msg": "Position PDA does not match Treasury PDA as owner"
    },
    {
      "code": 6023,
      "name": "FlashCpiFailed",
      "msg": "Flash Trade CPI call failed"
    },
    {
      "code": 6024,
      "name": "InvalidFlashProgramId",
      "msg": "Invalid Flash Trade program ID"
    },
    {
      "code": 6025,
      "name": "InvalidPoolName",
      "msg": "Pool name must be 1-32 characters"
    },
    {
      "code": 6026,
      "name": "CommittedDeltaExceedsBalance",
      "msg": "Decremented committed_sol_lamports exceeds tracked balance"
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
            "name": "beta_expires_at",
            "docs": [
              "Beta expiry: Unix timestamp after which the swarm stops managing this adopter.",
              "0 = permanent adopter (no expiry). Non-zero = beta adopter with sunset date."
            ],
            "type": "i64"
          },
          {
            "name": "beta_ended",
            "docs": [
              "Whether this beta has been manually ended by the authority"
            ],
            "type": "bool"
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
      "name": "BetaEnded",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "token_mint",
            "type": "pubkey"
          },
          {
            "name": "ended_at",
            "type": "i64"
          },
          {
            "name": "fees_contributed_lamports",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "EmergencyPositionsReset",
      "docs": [
        "Emitted by `emergency_close_all_positions`. Distinct from `FlashPositionClosed`",
        "because the on-chain instruction does NOT itself fire Flash Trade CPI close",
        "calls \u2014 it resets the position counters and records the operator's intent.",
        "Operators MUST follow up with explicit `close_flash_position` calls (or rely",
        "on Flash Trade liquidation) to actually close the on-chain positions."
      ],
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
            "name": "authority",
            "type": "pubkey"
          },
          {
            "name": "position_pubkeys",
            "type": {
              "vec": "pubkey"
            }
          },
          {
            "name": "previous_committed_sol_lamports",
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
      "name": "FlashOraclePrice",
      "docs": [
        "Oracle price \u2014 matches Flash Trade on-chain struct (i64 price, i32 exponent)",
        "Pyth uses exponent -8 (not -6 as originally assumed)"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "price",
            "type": "i64"
          },
          {
            "name": "exponent",
            "type": "i32"
          }
        ]
      }
    },
    {
      "name": "FlashPositionClosed",
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
            "name": "position_pda",
            "type": "pubkey"
          },
          {
            "name": "realised_pnl_sol_lamports",
            "type": "i64"
          },
          {
            "name": "returned_sol_lamports",
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
      "name": "FlashPositionOpened",
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
            "name": "side",
            "type": {
              "defined": {
                "name": "FlashSide"
              }
            }
          },
          {
            "name": "input_sol_lamports",
            "type": "u64"
          },
          {
            "name": "leverage_bps",
            "type": "u32"
          },
          {
            "name": "pool_name",
            "type": "string"
          },
          {
            "name": "position_pda",
            "type": "pubkey"
          },
          {
            "name": "ts",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "FlashSide",
      "docs": [
        "Position side \u2014 matches Flash Trade on-chain repr (None=0, Long=1, Short=2)"
      ],
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "None"
          },
          {
            "name": "Long"
          },
          {
            "name": "Short"
          }
        ]
      }
    },
    {
      "name": "Phase",
      "docs": [
        "Treasury phase -- can only advance forward. Transitions are IRREVERSIBLE.",
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
      "name": "Redistribution",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "excess",
            "type": "u64"
          },
          {
            "name": "holders_amount",
            "type": "u64"
          },
          {
            "name": "dev_amount",
            "type": "u64"
          },
          {
            "name": "ecosystem_amount",
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
            "name": "open_position_count",
            "docs": [
              "Number of currently open Flash Trade positions (max 3)"
            ],
            "type": "u8"
          },
          {
            "name": "committed_sol_lamports",
            "docs": [
              "Cumulative SOL (lamports) committed across all open positions"
            ],
            "type": "u64"
          },
          {
            "name": "flash_pool_name",
            "docs": [
              "Flash Trade pool identifier for this strategy (e.g., \"Crypto.1\")"
            ],
            "type": "string"
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
              "Current evolution phase (Sustenance \u2192 Ecosystem \u2192 Humanity)"
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
            "name": "frozen",
            "docs": [
              "Whether the treasury is frozen (emergency halt).",
              "When true, all non-read operations are rejected."
            ],
            "type": "bool"
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
      "name": "TreasuryFrozen",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "authority",
            "type": "pubkey"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "TreasuryUnfrozen",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "authority",
            "type": "pubkey"
          },
          {
            "name": "timestamp",
            "type": "i64"
          }
        ]
      }
    }
  ]
};
