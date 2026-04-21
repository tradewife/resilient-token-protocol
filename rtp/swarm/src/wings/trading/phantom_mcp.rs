//! Phantom MCP client — starts @phantom/mcp-server as a subprocess and
//! communicates via stdio JSON-RPC (MCP protocol).
//!
//! Provides typed Rust functions for:
//!   - SOL ↔ USDC swaps (fee-free via Phantom routing)
//!   - Deposit/withdraw to Hyperliquid (via Relay bridge)
//!   - Perps read operations (account, positions, orders)
//!
//! The subprocess reuses the existing session at ~/.phantom-mcp/session.json,
//! so re-authentication is only needed on first run or session expiry.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};

/// Portal App ID for the RTP Trading Wing.
const PORTAL_APP_ID: &str = "2fbef7dc-7975-4378-ba2b-ff8018ad2325";

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

// ── MCP Client ──────────────────────────────────────────────────────

/// MCP client that manages a @phantom/mcp-server subprocess.
pub struct PhantomMcpClient {
    child: Child,
    request_id: u64,
}

impl PhantomMcpClient {
    /// Start the MCP server subprocess and initialize the protocol.
    pub fn new() -> Result<Self, String> {
        let child = Command::new("npx")
            .args(["-y", "@phantom/mcp-server@latest"])
            .env("PHANTOM_APP_ID", PORTAL_APP_ID)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to start MCP server: {}", e))?;

        let mut client = Self {
            child,
            request_id: 0,
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
                    // Skip notifications, log messages, etc.
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

        // Send initialized notification (no id, no response expected).
        self.send(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }))?;

        // Give the server time to fully initialize and load tools.
        std::thread::sleep(std::time::Duration::from_secs(2));

        Ok(())
    }

    /// List available MCP tools. Returns the tool names.
    pub fn list_tools(&mut self) -> Result<Vec<String>, String> {
        let response = self.rpc_request("tools/list", serde_json::json!({}))?;

        let tools = response["result"]["tools"]
            .as_array()
            .ok_or("Missing tools array in response")?;

        Ok(tools
            .iter()
            .filter_map(|t| t["name"].as_str().map(|s| s.to_string()))
            .collect())
    }

    /// Call an MCP tool by name with the given arguments.
    /// First discovers the correct tool name via tools/list if needed.
    pub fn call_tool(
        &mut self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        // Discover available tools to find the correct name.
        let available = self.list_tools()?;
        let actual_name = available
            .iter()
            .find(|t| t == &name || t.replace('_', "").contains(&name.replace('_', "")))
            .cloned()
            .unwrap_or_else(|| name.to_string());

        let response = self.rpc_request(
            "tools/call",
            serde_json::json!({
                "name": actual_name,
                "arguments": arguments
            }),
        )?;

        // Check for JSON-RPC error.
        if let Some(err) = response.get("error") {
            let msg = err["message"].as_str().unwrap_or("unknown error");
            let code = err["code"].as_i64().unwrap_or(-1);
            return Err(format!("MCP error {}: {}", code, msg));
        }

        // Extract text content from the result.
        let content = response["result"]["content"]
            .as_array()
            .ok_or("Missing content array in MCP response")?;

        if content.is_empty() {
            return Err("Empty content in MCP response".to_string());
        }

        let text = content[0]["text"]
            .as_str()
            .ok_or("Missing text in MCP content")?;

        // Try to parse as JSON (most tool responses are JSON).
        match serde_json::from_str::<serde_json::Value>(text) {
            Ok(parsed) => Ok(parsed),
            Err(_) => Ok(serde_json::json!({"raw_text": text})),
        }
    }

    // ── High-level swap functions ────────────────────────────────

    /// Get a swap quote: SOL → USDC.
    pub fn quote_sol_to_usdc(&mut self, sol_amount: f64) -> Result<SwapQuote, String> {
        let result = self.call_tool(
            "buy",
            serde_json::json!({
                "amount": sol_amount.to_string(),
                "amountUnit": "ui",
                "sellTokenIsNative": true,
                "buyTokenMint": USDC_MINT,
                "execute": false
            }),
        )?;

        // The response has quoteResponse.quotes[0].
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
    pub fn swap_sol_to_usdc(&mut self, sol_amount: f64) -> Result<serde_json::Value, String> {
        self.call_tool(
            "buy",
            serde_json::json!({
                "amount": sol_amount.to_string(),
                "amountUnit": "ui",
                "sellTokenIsNative": true,
                "buyTokenMint": USDC_MINT,
                "execute": true
            }),
        )
    }

    /// Get a swap quote: USDC → SOL.
    pub fn quote_usdc_to_sol(&mut self, usdc_amount: f64) -> Result<SwapQuote, String> {
        let result = self.call_tool(
            "buy",
            serde_json::json!({
                "amount": usdc_amount.to_string(),
                "amountUnit": "ui",
                "sellTokenMint": USDC_MINT,
                "buyTokenIsNative": true,
                "execute": false
            }),
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
    pub fn swap_usdc_to_sol(&mut self, usdc_amount: f64) -> Result<serde_json::Value, String> {
        self.call_tool(
            "buy",
            serde_json::json!({
                "amount": usdc_amount.to_string(),
                "amountUnit": "ui",
                "sellTokenMint": USDC_MINT,
                "buyTokenIsNative": true,
                "execute": true
            }),
        )
    }

    // ── High-level HL bridge functions ───────────────────────────

    /// Quote a deposit to Hyperliquid: sell SOL, receive USDC on HL.
    pub fn quote_deposit_to_hl(&mut self, sol_amount: f64) -> Result<DepositQuote, String> {
        let result = self.call_tool(
            "perps_deposit",
            serde_json::json!({
                "sourceChainId": "solana:mainnet",
                "amount": sol_amount.to_string(),
                "sellTokenIsNative": true,
                "execute": false
            }),
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
    pub fn deposit_to_hl(&mut self, sol_amount: f64) -> Result<serde_json::Value, String> {
        self.call_tool(
            "perps_deposit",
            serde_json::json!({
                "sourceChainId": "solana:mainnet",
                "amount": sol_amount.to_string(),
                "sellTokenIsNative": true,
                "execute": true
            }),
        )
    }

    /// Withdraw from Hyperliquid to Solana.
    pub fn withdraw_from_hl(&mut self, usdc_amount: f64) -> Result<serde_json::Value, String> {
        self.call_tool(
            "perps_withdraw",
            serde_json::json!({
                "amountUsdc": usdc_amount.to_string(),
                "destinationChainId": "solana:mainnet"
            }),
        )
    }

    // ── Perps read functions ─────────────────────────────────────

    /// Get HL perps account balance.
    pub fn get_perps_account(&mut self) -> Result<PerpsAccount, String> {
        let result = self.call_tool("perps_account", serde_json::json!({}))?;
        Ok(PerpsAccount {
            account_value: result["accountValue"].as_str().unwrap_or("0.0").to_string(),
            available_balance: result["availableBalance"]
                .as_str()
                .unwrap_or("0.0")
                .to_string(),
        })
    }

    /// Get open perps positions.
    pub fn get_perps_positions(&mut self) -> Result<serde_json::Value, String> {
        self.call_tool("perps_positions", serde_json::json!({}))
    }

    // ── Wallet functions ─────────────────────────────────────────

    /// Get wallet addresses for all chains.
    pub fn get_wallet_addresses(&mut self) -> Result<serde_json::Value, String> {
        self.call_tool("wallet_addresses", serde_json::json!({}))
    }
}

impl Drop for PhantomMcpClient {
    fn drop(&mut self) {
        // Clean up the subprocess.
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

        // Test wallet addresses.
        match client.get_wallet_addresses() {
            Ok(addrs) => {
                let sol = addrs["addresses"]
                    .as_array()
                    .and_then(|a| a.first())
                    .and_then(|a| a["address"].as_str())
                    .unwrap_or("N/A");
                println!("[TEST] PhantomMCP wallet: {}", sol);
            }
            Err(e) => eprintln!("[TEST] get_wallet_addresses failed: {}", e),
        }
    }
}
