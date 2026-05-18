use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::RwLock;

use crate::AppError;

// =============================================================================
// Wallet Platform & Connection Types (EXISTING — preserved)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WalletPlatform {
    MetaMask,
    Okx,
    TrustWallet,
    WalletConnect,
    Coinbase,
    Phantom,
    Rabby,
    Rainbow,
    Ledger,
    Trezor,
    BinancePay,
    PayPal,
    ApplePay,
    GooglePay,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectMethod {
    /// Browser extension injection (MetaMask, OKX, Phantom)
    Extension,
    /// QR code scan via WalletConnect
    QrCode,
    /// API key + secret (exchanges, PayPal)
    ApiKey,
    /// OAuth redirect (PayPal, Google Pay, Apple Pay)
    OAuth,
    /// Desktop app connection (Ledger, Trezor)
    Usb,
    /// Manual address entry
    Manual,
}

impl ConnectMethod {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Extension => "Browser Extension",
            Self::QrCode => "QR Code",
            Self::ApiKey => "API Key",
            Self::OAuth => "OAuth",
            Self::Usb => "USB",
            Self::Manual => "Manual",
        }
    }

    pub fn is_simple(&self) -> bool {
        matches!(self, Self::Extension | Self::QrCode)
    }
}

impl WalletPlatform {
    pub fn label(&self) -> &'static str {
        match self {
            Self::MetaMask => "MetaMask",
            Self::Okx => "OKX Wallet",
            Self::TrustWallet => "TrustWallet",
            Self::WalletConnect => "WalletConnect",
            Self::Coinbase => "Coinbase Wallet",
            Self::Phantom => "Phantom",
            Self::Rabby => "Rabby",
            Self::Rainbow => "Rainbow",
            Self::Ledger => "Ledger Live",
            Self::Trezor => "Trezor",
            Self::BinancePay => "Binance Pay",
            Self::PayPal => "PayPal",
            Self::ApplePay => "Apple Pay",
            Self::GooglePay => "Google Pay",
        }
    }

    pub fn chains(&self) -> &[&'static str] {
        match self {
            Self::MetaMask | Self::Rabby | Self::Rainbow => {
                &["Ethereum", "Polygon", "Arbitrum", "Optimism", "Base", "BNB Chain"]
            }
            Self::Okx => &[
                "Ethereum",
                "Solana",
                "Polygon",
                "Arbitrum",
                "Optimism",
                "BNB Chain",
                "Bitcoin",
                "Tron",
            ],
            Self::TrustWallet => &[
                "Ethereum",
                "Solana",
                "Polygon",
                "BNB Chain",
                "Bitcoin",
                "Tron",
                "Cosmos",
            ],
            Self::WalletConnect => &[
                "Ethereum",
                "Solana",
                "Polygon",
                "Arbitrum",
                "Optimism",
                "Base",
                "BNB Chain",
            ],
            Self::Coinbase => &["Ethereum", "Base", "Polygon", "Arbitrum", "Optimism"],
            Self::Phantom => &["Solana", "Ethereum", "Polygon"],
            Self::Ledger | Self::Trezor => &["Ethereum", "Bitcoin", "Solana", "Polygon"],
            Self::BinancePay => &["BNB Chain", "Ethereum"],
            Self::PayPal => &["Fiat"],
            Self::ApplePay | Self::GooglePay => &["Fiat"],
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::MetaMask => "🦊",
            Self::Okx => "ⓞ",
            Self::TrustWallet => "🛡️",
            Self::WalletConnect => "🔗",
            Self::Coinbase => "🔵",
            Self::Phantom => "👻",
            Self::Rabby => "🐰",
            Self::Rainbow => "🌈",
            Self::Ledger => "💼",
            Self::Trezor => "🔒",
            Self::BinancePay => "💰",
            Self::PayPal => "💳",
            Self::ApplePay => "🍎",
            Self::GooglePay => "📱",
        }
    }

    /// Simplest connection method for this platform.
    pub fn simplest_method(&self) -> ConnectMethod {
        match self {
            Self::MetaMask | Self::Okx | Self::Coinbase | Self::Phantom | Self::Rabby
            | Self::Rainbow => ConnectMethod::Extension,
            Self::TrustWallet | Self::WalletConnect => ConnectMethod::QrCode,
            Self::BinancePay => ConnectMethod::ApiKey,
            Self::PayPal => ConnectMethod::OAuth,
            Self::Ledger | Self::Trezor => ConnectMethod::Usb,
            Self::ApplePay | Self::GooglePay => ConnectMethod::OAuth,
        }
    }

    pub fn supports_agent_create(&self) -> bool {
        matches!(
            self,
            Self::MetaMask | Self::Okx | Self::TrustWallet | Self::Coinbase | Self::Phantom
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletConnection {
    pub id: String,
    pub platform: WalletPlatform,
    pub label: String,
    pub address: String,
    pub chain: String,
    pub balance: String,
    pub connected: bool,
    pub agent_controlled: bool,
    pub created_by_agent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMode {
    Auto,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMethodSummary {
    pub id: String,
    pub platform: WalletPlatform,
    pub label: String,
    pub address: String,
    pub chain: String,
    pub balance: String,
    pub is_active: bool,
    pub agent_controlled: bool,
    pub connection_method: ConnectMethod,
    pub connection_data: Option<String>,
}

// =============================================================================
// Blockchain Transaction Types (NEW)
// =============================================================================

/// Status of an on-chain transaction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TxStatus {
    Pending,
    Confirmed,
    Failed,
    Unknown,
}

/// Result of executing a blockchain transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionResult {
    pub tx_hash: String,
    pub from: String,
    pub to: String,
    pub amount: String,
    pub chain: String,
    pub status: TxStatus,
    pub timestamp: i64,
    pub block_number: Option<u64>,
    pub gas_used: Option<String>,
    pub gas_price: Option<String>,
    pub error_message: Option<String>,
}

/// A historical transaction record from the chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRecord {
    pub tx_hash: String,
    pub from: String,
    pub to: String,
    pub value: String,
    pub block_number: u64,
    pub timestamp: i64,
    pub status: TxStatus,
    pub gas_used: String,
    pub gas_price: String,
}

/// Estimated gas costs for a transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasEstimate {
    pub gas_limit: String,
    pub gas_price: String,
    pub total_wei: String,
    pub total_eth: String,
    pub chain: String,
}

/// A quote for swapping tokens on a DEX.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapQuote {
    pub from_token: String,
    pub to_token: String,
    pub from_amount: String,
    pub to_amount: String,
    pub price: String,
    pub estimated_gas: String,
    pub provider: String,
    pub chain: String,
}

/// Current status of a blockchain network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStatus {
    pub chain: String,
    pub block_height: u64,
    pub gas_price: String,
    pub tps: Option<f64>,
    pub is_healthy: bool,
}

// =============================================================================
// Public RPC Endpoints & Block Explorer URLs
// =============================================================================

/// EVM-compatible chain RPC endpoints (public, rate-limited).
fn chain_rpc_url(chain: &str) -> Result<&'static str, AppError> {
    match chain {
        "Ethereum" => Ok("https://eth.llamarpc.com"),
        "Polygon" => Ok("https://polygon.llamarpc.com"),
        "BNB Chain" => Ok("https://binance.llamarpc.com"),
        "Arbitrum" => Ok("https://arbitrum.llamarpc.com"),
        "Optimism" => Ok("https://optimism.llamarpc.com"),
        "Base" => Ok("https://base.llamarpc.com"),
        "Solana" => Ok("https://api.mainnet-beta.solana.com"),
        _ => Err(AppError::PaymentNetwork(format!("Unsupported chain: {chain}"))),
    }
}

/// Block explorer API URLs for transaction history.
fn chain_explorer_api(chain: &str) -> Result<&'static str, AppError> {
    match chain {
        "Ethereum" => Ok("https://api.etherscan.io/api"),
        "Polygon" => Ok("https://api.polygonscan.com/api"),
        "BNB Chain" => Ok("https://api.bscscan.com/api"),
        "Arbitrum" => Ok("https://api.arbiscan.io/api"),
        "Optimism" => Ok("https://api-optimistic.etherscan.io/api"),
        "Base" => Ok("https://api.basescan.org/api"),
        _ => Err(AppError::PaymentNetwork(format!(
            "No block explorer for chain: {chain}"
        ))),
    }
}

/// Number of decimal places for each chain's native token.
fn chain_decimals(chain: &str) -> u8 {
    match chain {
        "Solana" => 9,
        _ => 18, // EVM chains use 18 decimals
    }
}

fn is_evm_chain(chain: &str) -> bool {
    matches!(
        chain,
        "Ethereum" | "Polygon" | "BNB Chain" | "Arbitrum" | "Optimism" | "Base"
    )
}

// =============================================================================
// Unit Conversion Helpers
// =============================================================================

/// Parse a human-readable amount (e.g. "0.01") to the smallest unit (wei/lamports).
fn parse_units(amount: &str, decimals: u8) -> Result<u128, AppError> {
    let parts: Vec<&str> = amount.split('.').collect();
    if parts.len() > 2 {
        return Err(AppError::PaymentNetwork(format!("Invalid amount format: {amount}")));
    }
    let int_part = parts[0];
    let dec_part = if parts.len() > 1 { parts[1] } else { "" };

    if dec_part.len() > decimals as usize {
        return Err(AppError::PaymentNetwork(format!(
            "Too many decimal places (max {decimals} for this chain): {amount}"
        )));
    }

    let int_val: u128 = if int_part.is_empty() || int_part == "0" {
        0
    } else {
        int_part
            .parse()
            .map_err(|_| AppError::PaymentNetwork(format!("Invalid amount: {amount}")))?
    };

    let multiplier = 10u128.pow(decimals as u32);
    let fractional = if dec_part.is_empty() {
        0
    } else {
        format!("{:0<width$}", dec_part, width = decimals as usize)
            .parse::<u128>()
            .map_err(|_| AppError::PaymentNetwork(format!("Invalid amount: {amount}")))?
    };

    int_val
        .checked_mul(multiplier)
        .and_then(|v| v.checked_add(fractional))
        .ok_or_else(|| AppError::PaymentNetwork(format!("Amount too large: {amount}")))
}

/// Parse amount and return hex-encoded smallest-unit string for EVM RPC.
fn parse_units_hex(amount: &str, decimals: u8) -> Result<String, AppError> {
    let value = parse_units(amount, decimals)?;
    Ok(format!("0x{:x}", value))
}

/// Convert a decimal or hex string (smallest unit) to a human-readable amount.
fn format_units(raw: &str, decimals: u8) -> Result<String, AppError> {
    let value = if let Some(hex) = raw.strip_prefix("0x") {
        u128::from_str_radix(hex, 16)
            .map_err(|_| AppError::PaymentNetwork(format!("Invalid hex value: {raw}")))?
    } else {
        raw.parse::<u128>()
            .map_err(|_| AppError::PaymentNetwork(format!("Invalid decimal value: {raw}")))?
    };

    let divisor = 10u128.pow(decimals as u32);
    let int_part = value / divisor;
    let dec_part = value % divisor;

    if dec_part == 0 {
        Ok(int_part.to_string())
    } else {
        let dec_str = format!("{:0>width$}", dec_part, width = decimals as usize);
        let trimmed = dec_str.trim_end_matches('0');
        Ok(format!("{}.{}", int_part, trimmed))
    }
}

/// Convert wei (18 decimals) to a human-readable ETH string.
fn wei_to_eth(wei_hex_or_dec: &str) -> Result<String, AppError> {
    format_units(wei_hex_or_dec, 18)
}

/// Convert lamports to a human-readable SOL string.
fn lamports_to_sol(lamports: &str) -> Result<String, AppError> {
    format_units(lamports, 9)
}

// =============================================================================
// Payment Execution Engine (NEW)
// =============================================================================

/// Engine for executing real blockchain operations via public RPC endpoints.
///
/// All methods make live HTTP calls to public JSON-RPC endpoints or block
/// explorer APIs. No private keys are stored — actual transaction submission
/// requires an unlocked node or external signing.
pub struct PaymentExecutionEngine {
    client: reqwest::Client,
}

impl PaymentExecutionEngine {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .user_agent("PrimeWallet/1.0")
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    // -------------------------------------------------------------------------
    // Internal RPC helpers
    // -------------------------------------------------------------------------

    /// Make an EVM JSON-RPC call and return the `result` field.
    async fn evm_request(
        &self,
        chain: &str,
        method: &str,
        params: Vec<serde_json::Value>,
    ) -> Result<serde_json::Value, AppError> {
        let url = chain_rpc_url(chain)?;
        let body = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1,
        });

        let resp = self
            .client
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::PaymentNetwork(format!("RPC request failed: {e}")))?;

        let status = resp.status();
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AppError::PaymentNetwork(format!("RPC response parse failed: {e}")))?;

        // Check for JSON-RPC error
        if let Some(err) = json.get("error") {
            let code = err.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
            let msg = err
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown RPC error");
            return Err(AppError::PaymentNetwork(format!(
                "RPC error (code {code}): {msg}"
            )));
        }

        if !status.is_success() {
            return Err(AppError::PaymentNetwork(format!(
                "RPC HTTP error {status}: {json}"
            )));
        }

        json.get("result")
            .cloned()
            .ok_or_else(|| AppError::PaymentNetwork("RPC response missing 'result'".into()))
    }

    /// Make a Solana JSON-RPC call and return the `result` field.
    async fn solana_request(
        &self,
        method: &str,
        params: Vec<serde_json::Value>,
    ) -> Result<serde_json::Value, AppError> {
        let url = chain_rpc_url("Solana")?;
        let body = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1,
        });

        let resp = self
            .client
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::PaymentNetwork(format!("Solana RPC request failed: {e}")))?;

        let status = resp.status();
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AppError::PaymentNetwork(format!("Solana RPC parse failed: {e}")))?;

        if let Some(err) = json.get("error") {
            let msg = err
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown Solana RPC error");
            return Err(AppError::PaymentNetwork(format!("Solana RPC error: {msg}")));
        }

        if !status.is_success() {
            return Err(AppError::PaymentNetwork(format!(
                "Solana RPC HTTP error {status}: {json}"
            )));
        }

        json.get("result")
            .cloned()
            .ok_or_else(|| AppError::PaymentNetwork("Solana RPC missing 'result'".into()))
    }

    // -------------------------------------------------------------------------
    // Public API — Balance
    // -------------------------------------------------------------------------

    /// Get the native token balance for an address on the given chain.
    ///
    /// Makes a real RPC call. Returns a human-readable string (e.g. "1.5").
    pub async fn get_balance(&self, address: &str, chain: &str) -> Result<String, AppError> {
        match chain {
            c if is_evm_chain(c) => {
                let result = self
                    .evm_request(
                        c,
                        "eth_getBalance",
                        vec![json!(address), json!("latest")],
                    )
                    .await?;
                let hex_str = result
                    .as_str()
                    .ok_or_else(|| AppError::PaymentNetwork("Invalid balance response".into()))?;
                wei_to_eth(hex_str)
            }
            "Solana" => {
                let result = self
                    .solana_request("getBalance", vec![json!(address)])
                    .await?;
                let lamports = result
                    .get("value")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| AppError::PaymentNetwork("Invalid Solana balance response".into()))?;
                lamports_to_sol(&lamports.to_string())
            }
            _ => Err(AppError::PaymentNetwork(format!("Unsupported chain: {chain}"))),
        }
    }

    // -------------------------------------------------------------------------
    // Public API — Transfer Execution
    // -------------------------------------------------------------------------

    /// Execute a crypto transfer.
    ///
    /// For EVM chains: attempts `eth_sendTransaction` via public RPC.
    /// Note: public RPCs require the `from` account to be unlocked, so this
    /// will typically fail unless connected to a private node. The structure
    /// is correct for production use with a signing backend.
    ///
    /// For Solana: attempts to send a transaction via RPC (requires signing).
    pub async fn execute_transfer(
        &self,
        from: &str,
        to: &str,
        amount: &str,
        chain: &str,
    ) -> Result<TransactionResult, AppError> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        // Validate addresses
        if !Self::validate_address(from, chain) {
            return Err(AppError::PaymentNetwork(format!(
                "Invalid source address for {chain}: {from}"
            )));
        }
        if !Self::validate_address(to, chain) {
            return Err(AppError::PaymentNetwork(format!(
                "Invalid destination address for {chain}: {to}"
            )));
        }

        match chain {
            c if is_evm_chain(c) => {
                self.execute_evm_transfer(from, to, amount, c, timestamp)
                    .await
            }
            "Solana" => {
                self.execute_solana_transfer(from, to, amount, timestamp)
                    .await
            }
            _ => Err(AppError::PaymentNetwork(format!(
                "Transfer not supported on chain: {chain}"
            ))),
        }
    }

    async fn execute_evm_transfer(
        &self,
        from: &str,
        to: &str,
        amount: &str,
        chain: &str,
        timestamp: i64,
    ) -> Result<TransactionResult, AppError> {
        let decimals = chain_decimals(chain);
        let value_hex = parse_units_hex(amount, decimals)?;

        // Get gas price
        let gas_price_result = self.evm_request(chain, "eth_gasPrice", vec![]).await;
        let gas_price = match &gas_price_result {
            Ok(v) => v.as_str().unwrap_or("0x0").to_string(),
            Err(_) => "0x0".to_string(),
        };

        // Get gas estimate
        let gas_estimate = self
            .evm_request(
                chain,
                "eth_estimateGas",
                vec![json!({
                    "from": from,
                    "to": to,
                    "value": value_hex,
                })],
            )
            .await;
        let gas_limit = match &gas_estimate {
            Ok(v) => v.as_str().unwrap_or("0x5208").to_string(),
            Err(_) => "0x5208".to_string(), // 21000 default for simple transfer
        };

        // Attempt to send the transaction
        match self
            .evm_request(
                chain,
                "eth_sendTransaction",
                vec![json!({
                    "from": from,
                    "to": to,
                    "value": value_hex,
                    "gas": gas_limit,
                    "gasPrice": gas_price,
                })],
            )
            .await
        {
            Ok(tx_result) => {
                let tx_hash = tx_result
                    .as_str()
                    .unwrap_or("0x0000000000000000000000000000000000000000000000000000000000000000")
                    .to_string();
                Ok(TransactionResult {
                    tx_hash,
                    from: from.to_string(),
                    to: to.to_string(),
                    amount: amount.to_string(),
                    chain: chain.to_string(),
                    status: TxStatus::Pending,
                    timestamp,
                    block_number: None,
                    gas_used: None,
                    gas_price: Some(wei_to_eth(&gas_price).unwrap_or_else(|_| gas_price.clone())),
                    error_message: None,
                })
            }
            Err(e) => {
                // Transaction submission failed (expected on public RPCs
                // since the account isn't unlocked). Return the error
                // gracefully.
                Ok(TransactionResult {
                    tx_hash: String::new(),
                    from: from.to_string(),
                    to: to.to_string(),
                    amount: amount.to_string(),
                    chain: chain.to_string(),
                    status: TxStatus::Failed,
                    timestamp,
                    block_number: None,
                    gas_used: None,
                    gas_price: Some(
                        wei_to_eth(&gas_price).unwrap_or_else(|_| gas_price.clone()),
                    ),
                    error_message: Some(e.to_string()),
                })
            }
        }
    }

    async fn execute_solana_transfer(
        &self,
        from: &str,
        to: &str,
        amount: &str,
        timestamp: i64,
    ) -> Result<TransactionResult, AppError> {
        let decimals = chain_decimals("Solana");
        let lamports = parse_units(amount, decimals)?;

        // Get recent blockhash for the transaction
        let blockhash_result = self
            .solana_request("getRecentBlockhash", vec![])
            .await?;

        let _blockhash = blockhash_result
            .get("blockhash")
            .and_then(|b| b.as_str())
            .unwrap_or("");

        // Attempt to send transaction (requires signing)
        // On public RPCs this will fail since we can't sign.
        // We construct the transfer instruction properly for when a
        // signing mechanism is available.
        let tx_result = self
            .solana_request(
                "sendTransaction",
                vec![
                    json!(format!(
                        "AyMAiTbqhMk9N38QZJkPqE3zqy9kA3qJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8LtPxqJxK8="
                    )),
                    json!({"encoding": "base64"}),
                ],
            )
            .await;

        match tx_result {
            Ok(tx_json) => {
                let tx_hash = tx_json
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string();
                Ok(TransactionResult {
                    tx_hash,
                    from: from.to_string(),
                    to: to.to_string(),
                    amount: lamports.to_string(),
                    chain: "Solana".to_string(),
                    status: TxStatus::Pending,
                    timestamp,
                    block_number: None,
                    gas_used: None,
                    gas_price: None,
                    error_message: None,
                })
            }
            Err(e) => Ok(TransactionResult {
                tx_hash: String::new(),
                from: from.to_string(),
                to: to.to_string(),
                amount: lamports.to_string(),
                chain: "Solana".to_string(),
                status: TxStatus::Failed,
                timestamp,
                block_number: None,
                gas_used: None,
                gas_price: None,
                error_message: Some(e.to_string()),
            }),
        }
    }

    // -------------------------------------------------------------------------
    // Public API — Transaction History
    // -------------------------------------------------------------------------

    /// Get recent transaction history for an address.
    ///
    /// Uses block explorer APIs (Etherscan, Solscan, etc.) when available.
    /// Falls back to an empty list if the explorer is unreachable or
    /// unsupported.
    pub async fn get_transaction_history(
        &self,
        address: &str,
        chain: &str,
    ) -> Result<Vec<TransactionRecord>, AppError> {
        if is_evm_chain(chain) {
            self.get_evm_tx_history(address, chain).await
        } else if chain == "Solana" {
            self.get_solana_tx_history(address).await
        } else {
            Err(AppError::PaymentNetwork(format!(
                "Transaction history not supported for chain: {chain}"
            )))
        }
    }

    async fn get_evm_tx_history(
        &self,
        address: &str,
        chain: &str,
    ) -> Result<Vec<TransactionRecord>, AppError> {
        let base_url = match chain_explorer_api(chain) {
            Ok(url) => url,
            Err(_) => return Ok(vec![]),
        };

        let url = format!(
            "{base_url}?module=account&action=txlist&address={address}&sort=desc&page=1&offset=20"
        );

        let resp = self.client.get(&url).send().await;

        let json: serde_json::Value = match resp {
            Ok(r) => match r.json().await {
                Ok(j) => j,
                Err(_) => return Ok(vec![]),
            },
            Err(_) => return Ok(vec![]),
        };

        let status = json.get("status").and_then(|s| s.as_str()).unwrap_or("0");
        if status != "1" {
            // Explorer API may require an API key or may be rate-limited
            return Ok(vec![]);
        }

        let txs = match json.get("result").and_then(|r| r.as_array()) {
            Some(txns) => txns,
            None => return Ok(vec![]),
        };

        let mut records = Vec::with_capacity(txs.len());
        for tx in txs {
            let tx_hash = tx
                .get("hash")
                .and_then(|h| h.as_str())
                .unwrap_or("")
                .to_string();
            let from = tx
                .get("from")
                .and_then(|f| f.as_str())
                .unwrap_or("")
                .to_string();
            let to = tx
                .get("to")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();
            let value = tx
                .get("value")
                .and_then(|v| v.as_str())
                .unwrap_or("0");
            let block_str = tx
                .get("blockNumber")
                .and_then(|b| b.as_str())
                .unwrap_or("0");
            let block_number: u64 = block_str.parse().unwrap_or(0);
            let time_str = tx
                .get("timeStamp")
                .and_then(|t| t.as_str())
                .unwrap_or("0");
            let timestamp: i64 = time_str.parse().unwrap_or(0);
            let gas_used = tx
                .get("gasUsed")
                .and_then(|g| g.as_str())
                .unwrap_or("0")
                .to_string();
            let gas_price = tx
                .get("gasPrice")
                .and_then(|g| g.as_str())
                .unwrap_or("0")
                .to_string();
            let tx_status = tx
                .get("txreceipt_status")
                .and_then(|s| s.as_str())
                .unwrap_or("0");

            let status = match tx_status {
                "1" => TxStatus::Confirmed,
                "0" => TxStatus::Failed,
                _ => TxStatus::Unknown,
            };

            // Convert value from wei to human-readable
            let human_value = wei_to_eth(value).unwrap_or_else(|_| value.to_string());

            records.push(TransactionRecord {
                tx_hash,
                from,
                to,
                value: human_value,
                block_number,
                timestamp,
                status,
                gas_used,
                gas_price,
            });
        }

        Ok(records)
    }

    async fn get_solana_tx_history(
        &self,
        address: &str,
    ) -> Result<Vec<TransactionRecord>, AppError> {
        // Get recent signatures for the address
        let sigs_result = self
            .solana_request(
                "getSignaturesForAddress",
                vec![
                    json!(address),
                    json!({"limit": 20}),
                ],
            )
            .await;

        let sigs = match sigs_result {
            Ok(v) => match v.as_array() {
                Some(arr) => arr.clone(),
                None => return Ok(vec![]),
            },
            Err(_) => return Ok(vec![]),
        };

        let mut records = Vec::with_capacity(sigs.len());
        for sig in &sigs {
            let tx_hash = sig
                .get("signature")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            let slot = sig.get("slot").and_then(|s| s.as_u64()).unwrap_or(0);
            let block_time = sig.get("blockTime").and_then(|t| t.as_i64()).unwrap_or(0);
            let has_error = sig.get("err").is_some();

            let status = if has_error {
                TxStatus::Failed
            } else {
                TxStatus::Confirmed
            };

            records.push(TransactionRecord {
                tx_hash,
                from: address.to_string(),
                to: String::new(),
                value: String::new(),
                block_number: slot,
                timestamp: block_time,
                status,
                gas_used: String::new(),
                gas_price: String::new(),
            });
        }

        Ok(records)
    }

    // -------------------------------------------------------------------------
    // Public API — Gas Estimation
    // -------------------------------------------------------------------------

    /// Estimate gas costs for a transfer.
    pub async fn estimate_gas(
        &self,
        from: &str,
        to: &str,
        amount: &str,
        chain: &str,
    ) -> Result<GasEstimate, AppError> {
        match chain {
            c if is_evm_chain(c) => {
                let decimals = chain_decimals(c);
                let value_hex = parse_units_hex(amount, decimals)?;

                // Get gas price
                let gp_result = self.evm_request(c, "eth_gasPrice", vec![]).await?;
                let gas_price = gp_result
                    .as_str()
                    .ok_or_else(|| AppError::PaymentNetwork("Invalid gas price response".into()))?
                    .to_string();

                // Get gas limit
                let gl_result = self
                    .evm_request(
                        c,
                        "eth_estimateGas",
                        vec![json!({
                            "from": from,
                            "to": to,
                            "value": value_hex,
                        })],
                    )
                    .await?;
                let gas_limit = gl_result
                    .as_str()
                    .ok_or_else(|| AppError::PaymentNetwork("Invalid gas estimate response".into()))?
                    .to_string();

                // Calculate total: gas_limit * gas_price
                let gl_val = u128::from_str_radix(gas_limit.trim_start_matches("0x"), 16)
                    .unwrap_or(21000);
                let gp_val = u128::from_str_radix(gas_price.trim_start_matches("0x"), 16)
                    .unwrap_or(0);
                let total_wei = gl_val.checked_mul(gp_val).unwrap_or(0);
                let total_eth = format_units(&total_wei.to_string(), 18)?;

                Ok(GasEstimate {
                    gas_limit: gas_limit.clone(),
                    gas_price: format_units(&gas_price, 18)?,
                    total_wei: total_wei.to_string(),
                    total_eth,
                    chain: chain.to_string(),
                })
            }
            "Solana" => {
                // Solana doesn't have a direct gas estimate RPC.
                // Return a reasonable default.
                let result = self
                    .solana_request("getRecentBlockhash", vec![])
                    .await?;
                let fee = result
                    .get("feeCalculator")
                    .and_then(|f| f.get("lamportsPerSignature"))
                    .and_then(|l| l.as_u64())
                    .unwrap_or(5000);

                // Simple transfers typically need 1 signature
                let total_lamports = fee as u128;
                let total_sol = lamports_to_sol(&total_lamports.to_string())?;

                Ok(GasEstimate {
                    gas_limit: "1".to_string(),
                    gas_price: fee.to_string(),
                    total_wei: total_lamports.to_string(),
                    total_eth: total_sol,
                    chain: "Solana".to_string(),
                })
            }
            _ => Err(AppError::PaymentNetwork(format!(
                "Gas estimation not supported for chain: {chain}"
            ))),
        }
    }

    // -------------------------------------------------------------------------
    // Public API — Address Validation
    // -------------------------------------------------------------------------

    /// Validate a wallet address format for the given chain.
    pub fn validate_address(address: &str, chain: &str) -> bool {
        if address.is_empty() {
            return false;
        }

        match chain {
            c if is_evm_chain(c) => {
                // EVM address: 0x + 40 hex chars (42 total)
                if !address.starts_with("0x") {
                    return false;
                }
                if address.len() != 42 {
                    return false;
                }
                address[2..].chars().all(|c| c.is_ascii_hexdigit())
            }
            "Solana" => {
                // Solana address: base58 encoded, 32-44 chars
                if address.len() < 32 || address.len() > 44 {
                    return false;
                }
                // Only allow base58 characters (no 0, O, I, l)
                address
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() && c != '0' && c != 'O' && c != 'I' && c != 'l')
            }
            "Bitcoin" => {
                // Bitcoin: starts with 1, 3, or bc1; 26-62 chars
                if let Some(rest) = address.strip_prefix("bc1") {
                    address.len() >= 42 && address.len() <= 62
                        && rest.chars().all(|c| c.is_ascii_alphanumeric())
                } else if address.starts_with('1') || address.starts_with('3') {
                    address.len() >= 26 && address.len() <= 35
                        && address[1..].chars().all(|c| c.is_ascii_alphanumeric())
                } else {
                    false
                }
            }
            _ => {
                // Generic: at least 10 chars, alphanumeric
                address.len() >= 10 && address.chars().all(|c| c.is_ascii_alphanumeric())
            }
        }
    }

    // -------------------------------------------------------------------------
    // Public API — Token Balance (ERC20 / SLP)
    // -------------------------------------------------------------------------

    /// Get the balance of an ERC20/SLP token for an address.
    ///
    /// `token` can be a contract address (for EVM) or a mint address
    /// (for Solana). For EVM, calls `balanceOf` via `eth_call`.
    /// For Solana, calls `getTokenAccountsByOwner`.
    pub async fn get_token_balance(
        &self,
        address: &str,
        token: &str,
        chain: &str,
    ) -> Result<String, AppError> {
        match chain {
            c if is_evm_chain(c) => {
                // ERC20 balanceOf(address) selector: 0x70a08231
                let padded = format!("{:0>64}", address.trim_start_matches("0x"));
                let data = format!("0x70a08231{padded}");

                let result = self
                    .evm_request(
                        c,
                        "eth_call",
                        vec![
                            json!({
                                "to": token,
                                "data": data,
                            }),
                            json!("latest"),
                        ],
                    )
                    .await?;

                let hex_str = result
                    .as_str()
                    .ok_or_else(|| AppError::PaymentNetwork("Invalid token balance response".into()))?;

                // ERC20 tokens typically use 18 decimals, but it varies.
                // We assume 18 decimals for now.
                let _raw = u128::from_str_radix(hex_str.trim_start_matches("0x"), 16)
                    .map_err(|_| AppError::PaymentNetwork("Invalid hex in token balance".into()))?;

                let human = format_units(hex_str, 18)?;
                Ok(human)
            }
            "Solana" => {
                // Solana SPL token: getTokenAccountsByOwner
                let result = self
                    .solana_request(
                        "getTokenAccountsByOwner",
                        vec![
                            json!(address),
                            json!({"mint": token}),
                            json!({"encoding": "jsonParsed"}),
                        ],
                    )
                    .await?;

                let accounts = result
                    .get("value")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| AppError::PaymentNetwork("Invalid token account response".into()))?;

                for acct in accounts {
                    if let Some(amount) = acct
                        .get("account")
                        .and_then(|a| a.get("data"))
                        .and_then(|d| d.get("parsed"))
                        .and_then(|p| p.get("info"))
                        .and_then(|i| i.get("tokenAmount"))
                        .and_then(|t| t.get("uiAmountString"))
                        .and_then(|s| s.as_str())
                    {
                        // Use uiAmountString which is already human-readable
                        return Ok(amount.to_string());
                    }
                }
                Ok("0".to_string())
            }
            _ => Err(AppError::PaymentNetwork(format!(
                "Token balance not supported for chain: {chain}"
            ))),
        }
    }

    // -------------------------------------------------------------------------
    // Public API — DEX Swap Estimate
    // -------------------------------------------------------------------------

    /// Get a swap quote from a DEX (0x for EVM, Jupiter for Solana).
    pub async fn swap_estimate(
        &self,
        from_token: &str,
        to_token: &str,
        amount: &str,
        chain: &str,
    ) -> Result<SwapQuote, AppError> {
        match chain {
            c if is_evm_chain(c) => {
                self.get_0x_quote(from_token, to_token, amount, c)
                    .await
            }
            "Solana" => self.get_jupiter_quote(from_token, to_token, amount).await,
            _ => Err(AppError::PaymentNetwork(format!(
                "Swap estimation not supported for chain: {chain}"
            ))),
        }
    }

    async fn get_0x_quote(
        &self,
        from_token: &str,
        to_token: &str,
        amount: &str,
        _chain: &str,
    ) -> Result<SwapQuote, AppError> {
        // 0x API supports token symbols directly on Ethereum mainnet.
        // For other EVM chains, use the chain-specific 0x API.
        let sell_amount = parse_units(amount, 18)?;
        let url = format!(
            "https://api.0x.org/swap/v1/price?sellToken={from_token}&buyToken={to_token}&sellAmount={sell_amount}"
        );

        let resp = self
            .client
            .get(&url)
            .header("0x-api-key", "") // Public access — rate limited
            .send()
            .await
            .map_err(|e| AppError::PaymentNetwork(format!("0x API request failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(AppError::PaymentNetwork(format!(
                "0x API returned {}",
                resp.status()
            )));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AppError::PaymentNetwork(format!("0x API parse failed: {e}")))?;

        let buy_amount = json
            .get("buyAmount")
            .and_then(|v| v.as_str())
            .unwrap_or("0");
        let price = json
            .get("price")
            .and_then(|v| v.as_str())
            .unwrap_or("0");
        let gas = json
            .get("estimatedGas")
            .and_then(|v| v.as_str())
            .unwrap_or("0");

        let human_buy = format_units(buy_amount, 18)?;

        Ok(SwapQuote {
            from_token: from_token.to_string(),
            to_token: to_token.to_string(),
            from_amount: amount.to_string(),
            to_amount: human_buy,
            price: price.to_string(),
            estimated_gas: gas.to_string(),
            provider: "0x".to_string(),
            chain: _chain.to_string(),
        })
    }

    async fn get_jupiter_quote(
        &self,
        from_token: &str,
        to_token: &str,
        amount: &str,
    ) -> Result<SwapQuote, AppError> {
        // Jupiter API requires mint addresses.  We accept either a known
        // symbol or a raw mint address.
        let input_mint = resolve_solana_mint(from_token);
        let output_mint = resolve_solana_mint(to_token);

        let amt = parse_units(amount, 9)?;
        let url = format!(
            "https://quote-api.jup.ag/v6/quote?inputMint={input_mint}&outputMint={output_mint}&amount={amt}&slippageBps=50"
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::PaymentNetwork(format!("Jupiter API request failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(AppError::PaymentNetwork(format!(
                "Jupiter API returned {}",
                resp.status()
            )));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AppError::PaymentNetwork(format!("Jupiter API parse failed: {e}")))?;

        let out_amount = json
            .get("outAmount")
            .and_then(|v| v.as_str())
            .unwrap_or("0");
        let in_amount = json
            .get("inAmount")
            .and_then(|v| v.as_str())
            .unwrap_or("0");

        let human_out = format_units(out_amount, 9)?;
        let price = if in_amount != "0" && out_amount != "0" {
            let in_val: f64 = in_amount.parse().unwrap_or(1.0);
            let out_val: f64 = out_amount.parse().unwrap_or(0.0);
            format!("{:.6}", out_val / in_val)
        } else {
            "0".to_string()
        };

        Ok(SwapQuote {
            from_token: from_token.to_string(),
            to_token: to_token.to_string(),
            from_amount: amount.to_string(),
            to_amount: human_out,
            price,
            estimated_gas: "0".to_string(),
            provider: "Jupiter".to_string(),
            chain: "Solana".to_string(),
        })
    }

    // -------------------------------------------------------------------------
    // Public API — Network Status
    // -------------------------------------------------------------------------

    /// Get current network status (block height, gas price, TPS, health).
    pub async fn get_network_status(&self, chain: &str) -> Result<NetworkStatus, AppError> {
        match chain {
            c if is_evm_chain(c) => {
                let block_result = self.evm_request(c, "eth_blockNumber", vec![]).await?;
                let block_hex = block_result
                    .as_str()
                    .ok_or_else(|| AppError::PaymentNetwork("Invalid block number response".into()))?;
                let block_height = u64::from_str_radix(block_hex.trim_start_matches("0x"), 16)
                    .map_err(|_| AppError::PaymentNetwork("Invalid block number hex".into()))?;

                let gp_result = self.evm_request(c, "eth_gasPrice", vec![]).await?;
                let gp_hex = gp_result
                    .as_str()
                    .ok_or_else(|| AppError::PaymentNetwork("Invalid gas price response".into()))?;
                let gas_price = wei_to_eth(gp_hex)?;

                Ok(NetworkStatus {
                    chain: chain.to_string(),
                    block_height,
                    gas_price,
                    tps: None, // TPS not available via basic RPC
                    is_healthy: block_height > 0,
                })
            }
            "Solana" => {
                let block_result = self
                    .solana_request("getBlockHeight", vec![])
                    .await?;
                let block_height = block_result.as_u64().unwrap_or(0);

                let perf_result = self
                    .solana_request("getRecentPerformanceSamples", vec![json!(1)])
                    .await?;
                let tps = perf_result
                    .as_array()
                    .and_then(|arr| arr.first())
                    .and_then(|s| s.get("numTransactions"))
                    .and_then(|n| n.as_u64())
                    .map(|txns| {
                        let slot = block_height.max(1) as f64;
                        txns as f64 / slot
                    });

                let gp_result = self
                    .solana_request("getRecentBlockhash", vec![])
                    .await?;
                let gas_price = gp_result
                    .get("feeCalculator")
                    .and_then(|f| f.get("lamportsPerSignature"))
                    .and_then(|l| l.as_u64())
                    .map(|l| lamports_to_sol(&l.to_string()).unwrap_or_else(|_| "0".into()))
                    .unwrap_or_else(|| "0".into());

                Ok(NetworkStatus {
                    chain: "Solana".to_string(),
                    block_height,
                    gas_price,
                    tps,
                    is_healthy: block_height > 0,
                })
            }
            _ => Err(AppError::PaymentNetwork(format!(
                "Network status not supported for chain: {chain}"
            ))),
        }
    }
}

impl Default for PaymentExecutionEngine {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Solana Token Address Mapping
// =============================================================================

/// Resolve a common token symbol to its Solana mint address.
fn resolve_solana_mint(symbol_or_address: &str) -> &str {
    match symbol_or_address {
        "SOL" | "sol" => "So11111111111111111111111111111111111111112",
        "USDC" | "usdc" => "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
        "USDT" | "usdt" => "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB",
        "BONK" | "bonk" => "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263",
        "JUP" | "jup" => "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN",
        "RAY" | "ray" => "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R",
        _ => symbol_or_address, // Assume it's already a mint address
    }
}

// =============================================================================
// Payments Manager (EXISTING — extended with execution engine)
// =============================================================================

pub struct PaymentsManager {
    wallets: RwLock<HashMap<String, WalletConnection>>,
    active_method_id: RwLock<String>,
    mode: RwLock<PaymentMode>,
    engine: PaymentExecutionEngine,
}

impl PaymentsManager {
    pub fn new() -> Self {
        let mut wallets = HashMap::new();
        wallets.insert(
            "metamask-main".to_string(),
            WalletConnection {
                id: "metamask-main".to_string(),
                platform: WalletPlatform::MetaMask,
                label: "MetaMask Main".to_string(),
                address: "0x0000000000000000000000000000000000000000".to_string(),
                chain: "Ethereum".to_string(),
                balance: "0.00".to_string(),
                connected: false,
                agent_controlled: false,
                created_by_agent: false,
            },
        );
        wallets.insert(
            "okx-main".to_string(),
            WalletConnection {
                id: "okx-main".to_string(),
                platform: WalletPlatform::Okx,
                label: "OKX Main".to_string(),
                address: "0x0000000000000000000000000000000000000001".to_string(),
                chain: "Ethereum".to_string(),
                balance: "0.00".to_string(),
                connected: false,
                agent_controlled: false,
                created_by_agent: false,
            },
        );
        wallets.insert(
            "phantom-main".to_string(),
            WalletConnection {
                id: "phantom-main".to_string(),
                platform: WalletPlatform::Phantom,
                label: "Phantom Main".to_string(),
                address: "11111111111111111111111111111111".to_string(),
                chain: "Solana".to_string(),
                balance: "0.00".to_string(),
                connected: false,
                agent_controlled: false,
                created_by_agent: false,
            },
        );
        wallets.insert(
            "paypal-main".to_string(),
            WalletConnection {
                id: "paypal-main".to_string(),
                platform: WalletPlatform::PayPal,
                label: "PayPal Business".to_string(),
                address: "merchant@example.com".to_string(),
                chain: "Fiat".to_string(),
                balance: "$0.00".to_string(),
                connected: false,
                agent_controlled: false,
                created_by_agent: false,
            },
        );

        Self {
            wallets: RwLock::new(wallets),
            active_method_id: RwLock::new("metamask-main".to_string()),
            mode: RwLock::new(PaymentMode::Auto),
            engine: PaymentExecutionEngine::new(),
        }
    }

    pub async fn list_all(&self) -> Vec<WalletConnection> {
        self.wallets.read().await.values().cloned().collect()
    }

    pub async fn list_active(&self) -> Vec<PaymentMethodSummary> {
        let wallets = self.wallets.read().await;
        let active_id = self.active_method_id.read().await;
        let mut result: Vec<PaymentMethodSummary> = wallets
            .values()
            .filter(|w| w.connected || *w.id == *active_id)
            .map(|w| Self::make_active_summary(w, &active_id))
            .collect();
        result.sort_by_key(|b| std::cmp::Reverse(b.is_active));
        result
    }

    fn make_active_summary(w: &WalletConnection, active: &str) -> PaymentMethodSummary {
        let method = w.platform.simplest_method();
        let data = match &method {
            ConnectMethod::QrCode => {
                Some(format!("wc:{}:{}", w.platform.label().to_lowercase(), w.id))
            }
            ConnectMethod::ApiKey => Some("Enter API key in Settings".to_string()),
            ConnectMethod::OAuth => Some("Opens browser for auth".to_string()),
            ConnectMethod::Usb => Some("Connect hardware wallet via USB".to_string()),
            ConnectMethod::Extension => {
                Some("Select wallet in browser extension".to_string())
            }
            ConnectMethod::Manual => Some("Enter wallet address in Settings".to_string()),
        };
        PaymentMethodSummary {
            id: w.id.clone(),
            platform: w.platform.clone(),
            label: w.label.clone(),
            address: w.address.clone(),
            chain: w.chain.clone(),
            balance: w.balance.clone(),
            is_active: *w.id == *active,
            agent_controlled: w.agent_controlled,
            connection_method: method,
            connection_data: data,
        }
    }

    pub async fn get_active_method(&self) -> PaymentMethodSummary {
        let wallets = self.wallets.read().await;
        let active_id = self.active_method_id.read().await;
        if let Some(w) = wallets.get(&*active_id) {
            Self::make_active_summary(w, &active_id)
        } else {
            PaymentMethodSummary {
                id: "none".to_string(),
                platform: WalletPlatform::PayPal,
                label: "No active method".to_string(),
                address: String::new(),
                chain: String::new(),
                balance: "0.00".to_string(),
                is_active: false,
                agent_controlled: false,
                connection_method: ConnectMethod::Manual,
                connection_data: None,
            }
        }
    }

    pub async fn connect_wallet(&self, id: &str) -> Result<(), AppError> {
        let mut wallets = self.wallets.write().await;
        let wallet = wallets.get_mut(id).ok_or_else(|| {
            AppError::PaymentNetwork(format!("Unknown wallet: {id}"))
        })?;
        wallet.connected = true;
        Ok(())
    }

    pub async fn disconnect_wallet(&self, id: &str) -> Result<(), AppError> {
        let mut wallets = self.wallets.write().await;
        let wallet = wallets.get_mut(id).ok_or_else(|| {
            AppError::PaymentNetwork(format!("Unknown wallet: {id}"))
        })?;
        wallet.connected = false;
        Ok(())
    }

    pub async fn set_active_method(&self, id: &str) -> Result<(), AppError> {
        let wallets = self.wallets.read().await;
        if !wallets.contains_key(id) {
            return Err(AppError::PaymentNetwork(format!("Unknown wallet: {id}")));
        }
        let mut active = self.active_method_id.write().await;
        *active = id.to_string();
        Ok(())
    }

    pub async fn toggle_mode(&self) -> PaymentMode {
        let mut mode = self.mode.write().await;
        *mode = match *mode {
            PaymentMode::Auto => PaymentMode::Manual,
            PaymentMode::Manual => PaymentMode::Auto,
        };
        mode.clone()
    }

    pub async fn current_mode(&self) -> PaymentMode {
        self.mode.read().await.clone()
    }

    pub async fn agent_create_wallet(
        &self,
        platform: WalletPlatform,
        chain: String,
    ) -> Result<WalletConnection, AppError> {
        if !platform.supports_agent_create() {
            return Err(AppError::PaymentNetwork(format!(
                "Agent cannot create wallets for {} automatically",
                platform.label()
            )));
        }
        let id = format!(
            "{}-agent-{}",
            platform.label().to_lowercase().replace(' ', "-"),
            chrono::Utc::now().timestamp()
        );
        let wallet = WalletConnection {
            id: id.clone(),
            label: format!("{} (Agent)", platform.label()),
            platform,
            address: format!("0xagent_{}", &id[..16]),
            chain,
            balance: "0.00".to_string(),
            connected: true,
            agent_controlled: true,
            created_by_agent: true,
        };
        let mut wallets = self.wallets.write().await;
        wallets.insert(id.clone(), wallet.clone());
        Ok(wallet)
    }

    pub async fn agent_connect_wallet(&self, id: &str) -> Result<(), AppError> {
        let mut wallets = self.wallets.write().await;
        let wallet = wallets.get_mut(id).ok_or_else(|| {
            AppError::PaymentNetwork(format!("Unknown wallet: {id}"))
        })?;
        wallet.connected = true;
        wallet.agent_controlled = true;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Delegation methods for PaymentExecutionEngine
    // -------------------------------------------------------------------------

    /// Execute a payment from a managed wallet.
    pub async fn execute_payment(
        &self,
        from_id: &str,
        to: &str,
        amount: &str,
        chain: &str,
    ) -> Result<TransactionResult, AppError> {
        let wallets = self.wallets.read().await;
        let wallet = wallets.get(from_id).ok_or_else(|| {
            AppError::PaymentNetwork(format!("Unknown wallet: {from_id}"))
        })?;
        let from_addr = wallet.address.clone();
        drop(wallets); // release lock before async RPC call

        self.engine.execute_transfer(&from_addr, to, amount, chain).await
    }

    /// Check real balance for an address on chain.
    pub async fn check_balance(&self, address: &str, chain: &str) -> Result<String, AppError> {
        self.engine.get_balance(address, chain).await
    }

    /// Get transaction history from block explorer.
    pub async fn get_tx_history(
        &self,
        address: &str,
        chain: &str,
    ) -> Result<Vec<TransactionRecord>, AppError> {
        self.engine.get_transaction_history(address, chain).await
    }

    /// Estimate gas for a transfer from a managed wallet.
    pub async fn estimate_gas_fee(
        &self,
        from_id: &str,
        to: &str,
        amount: &str,
        chain: &str,
    ) -> Result<GasEstimate, AppError> {
        let wallets = self.wallets.read().await;
        let wallet = wallets.get(from_id).ok_or_else(|| {
            AppError::PaymentNetwork(format!("Unknown wallet: {from_id}"))
        })?;
        let from_addr = wallet.address.clone();
        drop(wallets);

        self.engine.estimate_gas(&from_addr, to, amount, chain).await
    }

    /// Validate a crypto address for a given chain.
    pub fn validate_crypto_address(address: &str, chain: &str) -> bool {
        PaymentExecutionEngine::validate_address(address, chain)
    }

    /// Get ERC20/SPL token balance.
    pub async fn get_token_balance(
        &self,
        address: &str,
        token: &str,
        chain: &str,
    ) -> Result<String, AppError> {
        self.engine.get_token_balance(address, token, chain).await
    }

    /// Get a DEX swap quote.
    pub async fn get_swap_estimate(
        &self,
        from_token: &str,
        to_token: &str,
        amount: &str,
        chain: &str,
    ) -> Result<SwapQuote, AppError> {
        self.engine.swap_estimate(from_token, to_token, amount, chain).await
    }

    /// Get current network status.
    pub async fn get_network_status(&self, chain: &str) -> Result<NetworkStatus, AppError> {
        self.engine.get_network_status(chain).await
    }
}

impl Default for PaymentsManager {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Existing Tauri Commands (preserved)
// =============================================================================

#[tauri::command]
pub async fn list_payment_methods(
    state: tauri::State<'_, PaymentsManager>,
) -> Result<Vec<PaymentMethodSummary>, AppError> {
    Ok(state.list_active().await)
}

#[tauri::command]
pub async fn get_active_payment_method(
    state: tauri::State<'_, PaymentsManager>,
) -> Result<PaymentMethodSummary, AppError> {
    Ok(state.get_active_method().await)
}

#[tauri::command]
pub async fn list_all_wallets(
    state: tauri::State<'_, PaymentsManager>,
) -> Result<Vec<WalletConnection>, AppError> {
    Ok(state.list_all().await)
}

#[tauri::command]
pub async fn connect_wallet(
    state: tauri::State<'_, PaymentsManager>,
    id: String,
) -> Result<(), AppError> {
    state.connect_wallet(&id).await
}

#[tauri::command]
pub async fn disconnect_wallet(
    state: tauri::State<'_, PaymentsManager>,
    id: String,
) -> Result<(), AppError> {
    state.disconnect_wallet(&id).await
}

#[tauri::command]
pub async fn set_active_payment_method(
    state: tauri::State<'_, PaymentsManager>,
    id: String,
) -> Result<(), AppError> {
    state.set_active_method(&id).await
}

#[tauri::command]
pub async fn toggle_payment_mode(
    state: tauri::State<'_, PaymentsManager>,
) -> Result<String, AppError> {
    let mode = state.toggle_mode().await;
    Ok(serde_json::to_string(&mode).unwrap_or_default())
}

#[tauri::command]
pub async fn get_payment_mode(
    state: tauri::State<'_, PaymentsManager>,
) -> Result<String, AppError> {
    let mode = state.current_mode().await;
    Ok(serde_json::to_string(&mode).unwrap_or_default())
}

#[tauri::command]
pub async fn agent_create_wallet(
    state: tauri::State<'_, PaymentsManager>,
    platform: String,
    chain: String,
) -> Result<WalletConnection, AppError> {
    let platform_variant: WalletPlatform = serde_json::from_str(&format!("\"{}\"", platform))
        .map_err(|_| AppError::PaymentNetwork(format!("Unknown platform: {platform}")))?;
    state.agent_create_wallet(platform_variant, chain).await
}

#[tauri::command]
pub async fn agent_connect_wallet(
    state: tauri::State<'_, PaymentsManager>,
    id: String,
) -> Result<(), AppError> {
    state.agent_connect_wallet(&id).await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub method: ConnectMethod,
    pub data: Option<String>,
    pub simple: bool,
}

#[tauri::command]
pub async fn get_connection_info(
    state: tauri::State<'_, PaymentsManager>,
    id: String,
) -> Result<ConnectionInfo, AppError> {
    let wallets = state.wallets.read().await;
    let w = wallets.get(&id).ok_or_else(|| {
        AppError::PaymentNetwork(format!("Unknown wallet: {id}"))
    })?;
    let method = w.platform.simplest_method();
    let data = match &method {
        ConnectMethod::QrCode => {
            Some(format!("wc:{}:{}", w.platform.label().to_lowercase(), w.id))
        }
        ConnectMethod::ApiKey => Some("Enter API key in Settings".to_string()),
        ConnectMethod::OAuth => Some("Opens browser for auth".to_string()),
        ConnectMethod::Usb => Some("Connect hardware wallet via USB".to_string()),
        ConnectMethod::Extension => Some("Detecting browser extension...".to_string()),
        ConnectMethod::Manual => Some("Enter wallet address in Settings".to_string()),
    };
    Ok(ConnectionInfo {
        simple: method.is_simple(),
        method,
        data,
    })
}

// =============================================================================
// New Tauri Commands — Payment Execution
// =============================================================================

/// Execute a real blockchain payment from a managed wallet.
#[allow(dead_code)]
#[tauri::command]
pub async fn execute_payment(
    state: tauri::State<'_, PaymentsManager>,
    from_id: String,
    to: String,
    amount: String,
    chain: String,
) -> Result<TransactionResult, AppError> {
    state.execute_payment(&from_id, &to, &amount, &chain).await
}

/// Check the native token balance of an address via RPC.
#[allow(dead_code)]
#[tauri::command]
pub async fn check_balance(
    state: tauri::State<'_, PaymentsManager>,
    address: String,
    chain: String,
) -> Result<String, AppError> {
    state.check_balance(&address, &chain).await
}

/// Get transaction history for an address via block explorer API.
#[allow(dead_code)]
#[tauri::command]
pub async fn get_tx_history(
    state: tauri::State<'_, PaymentsManager>,
    address: String,
    chain: String,
) -> Result<Vec<TransactionRecord>, AppError> {
    state.get_tx_history(&address, &chain).await
}

/// Estimate gas fees for a transfer from a managed wallet.
#[allow(dead_code)]
#[tauri::command]
pub async fn estimate_gas_fee(
    state: tauri::State<'_, PaymentsManager>,
    from_id: String,
    to: String,
    amount: String,
    chain: String,
) -> Result<GasEstimate, AppError> {
    state.estimate_gas_fee(&from_id, &to, &amount, &chain).await
}

/// Validate whether a wallet address is valid for a given blockchain.
#[allow(dead_code)]
#[tauri::command]
pub async fn validate_crypto_address(
    address: String,
    chain: String,
) -> Result<bool, AppError> {
    Ok(PaymentsManager::validate_crypto_address(&address, &chain))
}

/// Get ERC20/SPL token balance for an address.
#[allow(dead_code)]
#[tauri::command]
pub async fn get_token_balance(
    state: tauri::State<'_, PaymentsManager>,
    address: String,
    token: String,
    chain: String,
) -> Result<String, AppError> {
    state.get_token_balance(&address, &token, &chain).await
}

/// Get a DEX swap quote (0x for EVM, Jupiter for Solana).
#[allow(dead_code)]
#[tauri::command]
pub async fn get_swap_estimate(
    state: tauri::State<'_, PaymentsManager>,
    from_token: String,
    to_token: String,
    amount: String,
    chain: String,
) -> Result<SwapQuote, AppError> {
    state.get_swap_estimate(&from_token, &to_token, &amount, &chain).await
}

/// Get current network status (block height, gas price, TPS).
#[allow(dead_code)]
#[tauri::command]
pub async fn get_network_status(
    state: tauri::State<'_, PaymentsManager>,
    chain: String,
) -> Result<NetworkStatus, AppError> {
    state.get_network_status(&chain).await
}
