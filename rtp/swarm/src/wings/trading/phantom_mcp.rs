//! Phantom MCP client — starts @phantom/mcp-server as a subprocess and
//! communicates via stdio JSON-RPC (MCP protocol).
//!
//! Provides typed Rust functions for:
//!   - SOL ↔ USDC swaps (fee-free via Phantom routing)
//!   - Deposit/withdraw to Hyperliquid (via Relay bridge)
//!   - Perps trading (open/close positions, manage orders)
//!   - Yield distribution (transfer tokens to dev/holders/ecosystem)
//!   - Balance queries, wallet addresses
//!
//! Every function accepts a `di: u32` (derivation index) parameter.
//! Each token gets its own wallet at a unique index — isolated Solana
//! address, EVM address, and Hyperliquid account. One MCP auth session
//! supports unlimited per-token wallets.
//!
//! The subprocess reuses the existing session at ~/.phantom-mcp/session.json,
//! so re-authentication is only needed on first run or session expiry.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};

/// Solana mainnet USDC mint.
pub const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

/// MCP protocol version.
const MCP_VERSION: &str = "2024-11-05";

// ── Response types ──────────────────────────────────────────────────

/// Swap quote returned by Phantom's routing engine.
#[derive(Debug, serde::Deserialize)]
pub struct SwapQuote {
    pub buy_amount: String,
    pub sell_amount: String,
    pub price_impact: f64,
    pub slippage_tolerance: f64,
}

/// Perps account balance.
#[derive(Debug, serde::Deserialize)]
pub struct PerpsAccount {
    pub account_value: String,
    pub available_balance: String,
}

/// Result of a deposit quote (bridge to HL).
#[derive(Debug)]
pub struct DepositQuote {
    pub buy_amount_usdc: String,
    pub sell_amount_lamports: String,
    pub fees_total_lamports: String,
    pub relay_id: String,
}

/// Token balance entry from get_token_balances.
#[derive(Debug, serde::Deserialize)]
pub struct TokenBalance {
    pub caip19: String,
    pub total_quantity: String,
    pub symbol: String,
    pub name: String,
    #[serde(default)]
    pub price: Option<TokenPrice>,
}

/// Price data for a token balance.
#[derive(Debug, serde::Deserialize)]
pub struct TokenPrice {
    pub price: f64,
    #[serde(default)]
    pub price_change_24h: Option<f64>,
}

// ── MCP Client ──────────────────────────────────────────────────────

/// MCP client that manages a @phantom/mcp-server subprocess.
///
/// All operations accept a `di` (derivation index) parameter for per-token
/// wallet isolation. Index 0 is the default agent wallet, 1+ are assigned
/// to individual token treasuries as they register.
pub struct PhantomMcpClient {
    child: Child,
    request_id: u64,
    /// Cache of discovered tool names (populated on first call_tool).
    tool_names: Option<Vec<String>>,
}

impl PhantomMcpClient {
    /// Start the MCP server subprocess and initialize the protocol.
    pub fn new() -> Result<Self, String> {
        let child = Command::new("npx")
            .args(["-y", "@phantom/mcp-server@latest"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to start MCP server: {}", e))?;

        let mut client = Self {
            child,
            request_id: 0,
            tool_names: None,
        };

        client.initialize()?;
        tracing::info!("[PhantomMCP] subprocess started and initialized");
        Ok(client)
    }

    // ── MCP protocol ────────────────────────────────────────────

    fn next_id(&mut self) -> u64 {
        self.request_id += 1;
        self.request_id
    }

    fn send(&mut self, msg: &serde_json::Value) -> Result<(), String> {
        let stdin = self.child.stdin.as_mut().ok_or("stdin unavailable")?;
        let line = serde_json::to_string(msg).map_err(|e| format!("JSON encode: {}", e))?;
        writeln!(stdin, "{}", line).map_err(|e| format!("stdin write: {}", e))?;
        stdin.flush().map_err(|e| format!("stdin flush: {}", e))?;
        Ok(())
    }

    /// Read lines from stdout until we get a JSON-RPC response with the given ID.
    fn read_response(&mut self, expected_id: u64) -> Result<serde_json::Value, String> {
        let stdout = self.child.stdout.as_mut().ok_or("stdout unavailable")?;
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            let line = line.map_err(|e| format!("stdout read: {}", e))?;
            let trimmed = line.trim().to_string();
            if trimmed.is_empty() {
                continue;
            }
            match serde_json::from_str::<serde_json::Value>(&trimmed) {
                Ok(val) => {
                    if val.get("id").and_then(|v| v.as_u64()) == Some(expected_id) {
                        return Ok(val);
                    }
                }
                Err(_) => continue,
            }
        }
        Err("MCP server closed stdout".to_string())
    }

    fn rpc_request(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let id = self.next_id();
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });
        self.send(&msg)?;
        self.read_response(id)
    }

    /// Send MCP initialize handshake.
    fn initialize(&mut self) -> Result<(), String> {
        let _response = self.rpc_request(
            "initialize",
            serde_json::json!({
                "protocolVersion": MCP_VERSION,
                "capabilities": {},
                "clientInfo": {
                    "name": "rtp-trading-wing",
                    "version": "0.1.0"
                }
            }),
        )?;

        self.send(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }))?;

        std::thread::sleep(std::time::Duration::from_secs(2));
        Ok(())
    }

    /// List available MCP tools. Caches result for subsequent calls.
    pub fn list_tools(&mut self) -> Result<Vec<String>, String> {
        if let Some(ref names) = self.tool_names {
            return Ok(names.clone());
        }
        let response = self.rpc_request("tools/list", serde_json::json!({}))?;
        let tools = response["result"]["tools"]
            .as_array()
            .ok_or("Missing tools array in response")?;
        let names: Vec<String> = tools
            .iter()
            .filter_map(|t| t["name"].as_str().map(|s| s.to_string()))
            .collect();
        self.tool_names = Some(names.clone());
        Ok(names)
    }

    /// Resolve a logical tool name to the actual name exposed by the server.
    fn resolve_tool_name(&mut self, name: &str) -> String {
        if let Ok(available) = self.list_tools() {
            if let Some(exact) = available.iter().find(|t| *t == name) {
                return exact.clone();
            }
            // Fuzzy: match ignoring underscores.
            let name_flat = name.replace('_', "");
            if let Some(fuzzy) = available
                .iter()
                .find(|t| t.replace('_', "").contains(&name_flat))
            {
                return fuzzy.clone();
            }
        }
        name.to_string()
    }

    /// Call an MCP tool by name with derivation index injected into arguments.
    fn call_tool(
        &mut self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let actual_name = self.resolve_tool_name(name);

        let response = self.rpc_request(
            "tools/call",
            serde_json::json!({
                "name": actual_name,
                "arguments": arguments
            }),
        )?;

        if let Some(err) = response.get("error") {
            let msg = err["message"].as_str().unwrap_or("unknown error");
            let code = err["code"].as_i64().unwrap_or(-1);

            // Check for spending limit errors specifically.
            if msg.contains("SPENDING_LIMIT_EXCEEDED")
                || msg.contains("spending limit")
                || msg.contains("SpendingLimit")
            {
                tracing::error!(
                    "[PhantomMCP] SPENDING_LIMIT_EXCEEDED: {} — \
                     user must adjust on-chain spending limits in Phantom wallet",
                    msg
                );
            }

            return Err(format!("MCP error {}: {}", code, msg));
        }

        let content = response["result"]["content"]
            .as_array()
            .ok_or("Missing content array in MCP response")?;

        if content.is_empty() {
            return Err("Empty content in MCP response".to_string());
        }

        let text = content[0]["text"]
            .as_str()
            .ok_or("Missing text in MCP content")?;

        match serde_json::from_str::<serde_json::Value>(text) {
            Ok(parsed) => Ok(parsed),
            Err(_) => Ok(serde_json::json!({"raw_text": text})),
        }
    }

    /// Call an MCP tool with derivation index injected.
    fn call_tool_di(
        &mut self,
        name: &str,
        mut arguments: serde_json::Value,
        di: u32,
    ) -> Result<serde_json::Value, String> {
        // Inject derivationIndex into the arguments object.
        if let Some(obj) = arguments.as_object_mut() {
            obj.insert(
                "derivationIndex".to_string(),
                serde_json::Value::Number(serde_json::Number::from(di)),
            );
        }
        self.call_tool(name, arguments)
    }

    // ── High-level swap functions ────────────────────────────────

    /// Get a swap quote: SOL → USDC.
    pub fn quote_sol_to_usdc(&mut self, sol_amount: f64, di: u32) -> Result<SwapQuote, String> {
        let result = self.call_tool_di(
            "buy",
            serde_json::json!({
                "amount": sol_amount.to_string(),
                "amountUnit": "ui",
                "sellTokenIsNative": true,
                "buyTokenMint": USDC_MINT,
                "execute": false
            }),
            di,
        )?;

        let quotes = result["quoteResponse"]["quotes"]
            .as_array()
            .ok_or("Missing quotes in swap response")?;

        if quotes.is_empty() {
            return Err("No swap quotes available".to_string());
        }

        let best = &quotes[0];
        Ok(SwapQuote {
            buy_amount: best["buyAmount"].as_str().unwrap_or("0").to_string(),
            sell_amount: best["sellAmount"].as_str().unwrap_or("0").to_string(),
            price_impact: best["priceImpact"].as_f64().unwrap_or(0.0),
            slippage_tolerance: best["slippageTolerance"].as_f64().unwrap_or(0.0),
        })
    }

    /// Execute a swap: SOL → USDC.
    pub fn swap_sol_to_usdc(
        &mut self,
        sol_amount: f64,
        di: u32,
    ) -> Result<serde_json::Value, String> {
        self.call_tool_di(
            "buy",
            serde_json::json!({
                "amount": sol_amount.to_string(),
                "amountUnit": "ui",
                "sellTokenIsNative": true,
                "buyTokenMint": USDC_MINT,
                "execute": true
            }),
            di,
        )
    }

    /// Get a swap quote: USDC → SOL.
    pub fn quote_usdc_to_sol(&mut self, usdc_amount: f64, di: u32) -> Result<SwapQuote, String> {
        let result = self.call_tool_di(
            "buy",
            serde_json::json!({
                "amount": usdc_amount.to_string(),
                "amountUnit": "ui",
                "sellTokenMint": USDC_MINT,
                "buyTokenIsNative": true,
                "execute": false
            }),
            di,
        )?;

        let quotes = result["quoteResponse"]["quotes"]
            .as_array()
            .ok_or("Missing quotes in swap response")?;

        if quotes.is_empty() {
            return Err("No swap quotes available".to_string());
        }

        let best = &quotes[0];
        Ok(SwapQuote {
            buy_amount: best["buyAmount"].as_str().unwrap_or("0").to_string(),
            sell_amount: best["sellAmount"].as_str().unwrap_or("0").to_string(),
            price_impact: best["priceImpact"].as_f64().unwrap_or(0.0),
            slippage_tolerance: best["slippageTolerance"].as_f64().unwrap_or(0.0),
        })
    }

    /// Execute a swap: USDC → SOL.
    pub fn swap_usdc_to_sol(
        &mut self,
        usdc_amount: f64,
        di: u32,
    ) -> Result<serde_json::Value, String> {
        self.call_tool_di(
            "buy",
            serde_json::json!({
                "amount": usdc_amount.to_string(),
                "amountUnit": "ui",
                "sellTokenMint": USDC_MINT,
                "buyTokenIsNative": true,
                "execute": true
            }),
            di,
        )
    }

    // ── High-level HL bridge functions ───────────────────────────

    /// Quote a deposit to Hyperliquid: sell SOL, receive USDC on HL.
    pub fn quote_deposit_to_hl(
        &mut self,
        sol_amount: f64,
        di: u32,
    ) -> Result<DepositQuote, String> {
        let result = self.call_tool_di(
            "perps_deposit",
            serde_json::json!({
                "sourceChainId": "solana:mainnet",
                "amount": sol_amount.to_string(),
                "sellTokenIsNative": true,
                "execute": false
            }),
            di,
        )?;

        let quotes = result["quoteResponse"]["quotes"]
            .as_array()
            .ok_or("Missing quotes in deposit response")?;

        if quotes.is_empty() {
            return Err("No deposit quotes available".to_string());
        }

        let best = &quotes[0];
        let step = best["steps"].as_array().and_then(|s| s.first());

        let fees_total = step
            .and_then(|s| s["includedFeeCosts"].as_array())
            .map(|fees| {
                fees.iter()
                    .filter_map(|f| f["amount"].as_str().and_then(|a| a.parse::<u64>().ok()))
                    .sum::<u64>()
                    .to_string()
            })
            .unwrap_or_else(|| "0".to_string());

        Ok(DepositQuote {
            buy_amount_usdc: best["buyAmount"].as_str().unwrap_or("0").to_string(),
            sell_amount_lamports: best["sellAmount"].as_str().unwrap_or("0").to_string(),
            fees_total_lamports: fees_total,
            relay_id: best["baseProvider"]["id"]
                .as_str()
                .unwrap_or("unknown")
                .to_string(),
        })
    }

    /// Execute deposit to Hyperliquid.
    pub fn deposit_to_hl(&mut self, sol_amount: f64, di: u32) -> Result<serde_json::Value, String> {
        self.call_tool_di(
            "perps_deposit",
            serde_json::json!({
                "sourceChainId": "solana:mainnet",
                "amount": sol_amount.to_string(),
                "sellTokenIsNative": true,
                "execute": true
            }),
            di,
        )
    }

    /// Withdraw from Hyperliquid perps to Solana.
    pub fn withdraw_from_hl(
        &mut self,
        usdc_amount: f64,
        di: u32,
    ) -> Result<serde_json::Value, String> {
        self.call_tool_di(
            "perps_withdraw",
            serde_json::json!({
                "amountUsdc": usdc_amount.to_string(),
                "destinationChainId": "solana:mainnet"
            }),
            di,
        )
    }

    /// Transfer USDC from HL spot account to perps margin account.
    pub fn transfer_spot_to_perps(
        &mut self,
        amount_usdc: f64,
        di: u32,
    ) -> Result<serde_json::Value, String> {
        self.call_tool_di(
            "perps_transfer",
            serde_json::json!({
                "amountUsdc": amount_usdc.to_string()
            }),
            di,
        )
    }

    // ── Perps read functions ─────────────────────────────────────

    /// Get HL perps account balance.
    pub fn get_perps_account(&mut self, di: u32) -> Result<PerpsAccount, String> {
        let result = self.call_tool_di("perps_account", serde_json::json!({}), di)?;
        Ok(PerpsAccount {
            account_value: result["accountValue"].as_str().unwrap_or("0.0").to_string(),
            available_balance: result["availableBalance"]
                .as_str()
                .unwrap_or("0.0")
                .to_string(),
        })
    }

    /// Get open perps positions.
    pub fn get_perps_positions(&mut self, di: u32) -> Result<serde_json::Value, String> {
        self.call_tool_di("perps_positions", serde_json::json!({}), di)
    }

    /// Get open perps orders.
    pub fn get_perp_orders(&mut self, di: u32) -> Result<serde_json::Value, String> {
        self.call_tool_di("perps_orders", serde_json::json!({}), di)
    }

    /// Get perps trade history.
    pub fn get_perp_trade_history(&mut self, di: u32) -> Result<serde_json::Value, String> {
        self.call_tool_di("perps_history", serde_json::json!({}), di)
    }

    /// Get available perps markets (no derivation index needed — public data).
    pub fn get_perp_markets(&mut self) -> Result<serde_json::Value, String> {
        self.call_tool("perps_markets", serde_json::json!({}))
    }

    // ── Perps write functions ────────────────────────────────────

    /// Open a perpetual position.
    pub fn open_perp_position(
        &mut self,
        market: &str,
        direction: &str,
        size_usd: f64,
        leverage: u32,
        margin_type: &str,
        di: u32,
    ) -> Result<serde_json::Value, String> {
        self.call_tool_di(
            "perps_open",
            serde_json::json!({
                "market": market,
                "direction": direction,
                "sizeUsd": size_usd.to_string(),
                "leverage": leverage,
                "marginType": margin_type
            }),
            di,
        )
    }

    /// Close a perpetual position (full or partial).
    pub fn close_perp_position(
        &mut self,
        market: &str,
        size_percent: u32,
        di: u32,
    ) -> Result<serde_json::Value, String> {
        self.call_tool_di(
            "perps_close",
            serde_json::json!({
                "market": market,
                "sizePercent": size_percent
            }),
            di,
        )
    }

    /// Cancel an open perp order.
    pub fn cancel_perp_order(
        &mut self,
        market: &str,
        order_id: i64,
        di: u32,
    ) -> Result<serde_json::Value, String> {
        self.call_tool_di(
            "perps_cancel",
            serde_json::json!({
                "market": market,
                "orderId": order_id
            }),
            di,
        )
    }

    /// Update leverage and margin type for a market.
    pub fn update_perp_leverage(
        &mut self,
        market: &str,
        leverage: u32,
        margin_type: &str,
        di: u32,
    ) -> Result<serde_json::Value, String> {
        self.call_tool_di(
            "perps_leverage",
            serde_json::json!({
                "market": market,
                "leverage": leverage,
                "marginType": margin_type
            }),
            di,
        )
    }

    // ── Wallet functions ─────────────────────────────────────────

    /// Get wallet addresses for all chains at the given derivation index.
    pub fn get_wallet_addresses(&mut self, di: u32) -> Result<serde_json::Value, String> {
        self.call_tool_di("wallet_addresses", serde_json::json!({}), di)
    }

    /// Get token balances for the wallet at the given derivation index.
    pub fn get_token_balances(&mut self, di: u32) -> Result<serde_json::Value, String> {
        self.call_tool_di("get_token_balances", serde_json::json!({}), di)
    }

    /// Transfer tokens on Solana or EVM chains.
    pub fn transfer_tokens(
        &mut self,
        network_id: &str,
        to: &str,
        amount: &str,
        token_mint: Option<&str>,
        di: u32,
    ) -> Result<serde_json::Value, String> {
        let mut args = serde_json::json!({
            "networkId": network_id,
            "to": to,
            "amount": amount,
            "amountUnit": "ui"
        });

        if let Some(mint) = token_mint {
            args["tokenMint"] = serde_json::Value::String(mint.to_string());
        }

        self.call_tool_di("transfer_tokens", args, di)
    }

    /// Send a signed Solana transaction.
    pub fn send_solana_transaction(
        &mut self,
        transaction: &str,
        network_id: &str,
        di: u32,
    ) -> Result<serde_json::Value, String> {
        self.call_tool_di(
            "send_solana_transaction",
            serde_json::json!({
                "transaction": transaction,
                "networkId": network_id
            }),
            di,
        )
    }

    /// Simulate a transaction without submitting (preview asset changes).
    pub fn simulate_transaction(
        &mut self,
        chain_id: &str,
        tx_type: &str,
        params: serde_json::Value,
        di: u32,
    ) -> Result<serde_json::Value, String> {
        self.call_tool_di(
            "simulate",
            serde_json::json!({
                "chainId": chain_id,
                "type": tx_type,
                "params": params
            }),
            di,
        )
    }
}

impl Drop for PhantomMcpClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phantom_mcp_client_starts_and_initializes() {
        if std::env::var("SKIP_MCP_TESTS").is_ok() {
            eprintln!("SKIP phantom_mcp_client_starts_and_initializes (SKIP_MCP_TESTS)");
            return;
        }

        let mut client = match PhantomMcpClient::new() {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "SKIP phantom_mcp_client_starts_and_initializes (MCP init failed: {})",
                    e
                );
                return;
            }
        };

        // List available tools.
        match client.list_tools() {
            Ok(tools) => {
                println!("[TEST] Available MCP tools ({} total):", tools.len());
                for t in &tools {
                    println!("  - {}", t);
                }
            }
            Err(e) => eprintln!("[TEST] list_tools failed: {}", e),
        }

        // Test wallet addresses at index 0.
        match client.get_wallet_addresses(0) {
            Ok(addrs) => {
                let sol = addrs["addresses"]
                    .as_array()
                    .and_then(|a| a.first())
                    .and_then(|a| a["address"].as_str())
                    .unwrap_or("N/A");
                println!("[TEST] PhantomMCP wallet (di=0): {}", sol);
            }
            Err(e) => eprintln!("[TEST] get_wallet_addresses failed: {}", e),
        }

        // Test that derivation index 1 gives a different address.
        match client.get_wallet_addresses(1) {
            Ok(addrs) => {
                let sol = addrs["addresses"]
                    .as_array()
                    .and_then(|a| a.first())
                    .and_then(|a| a["address"].as_str())
                    .unwrap_or("N/A");
                println!("[TEST] PhantomMCP wallet (di=1): {}", sol);
            }
            Err(e) => eprintln!("[TEST] get_wallet_addresses (di=1) failed: {}", e),
        }
    }
}
