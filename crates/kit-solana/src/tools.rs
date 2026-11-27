//! kit-solana tools for the kit registry.
//!
//! Read tools are read-only. The transfer tool is consequential: in the
//! reasoning loop it passes the Jev gate before it can run, and in dry-run
//! it is simulated instead of executed.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};

use kit_core::{Tool, ToolError, ToolRegistry};

use crate::client::SolanaClient;
use crate::signer::SolanaSigner;
use crate::transfer::build_transfer;

fn parse_pubkey(args: &Value, field: &str) -> Result<solana_sdk::pubkey::Pubkey, ToolError> {
    let s = args[field].as_str().ok_or_else(|| ToolError::Failed {
        name: "solana".to_string(),
        message: format!("missing string argument `{field}`"),
    })?;
    s.parse().map_err(|e| ToolError::Failed {
        name: "solana".to_string(),
        message: format!("invalid pubkey in `{field}`: {e}"),
    })
}

fn rpc_tool_err(e: crate::SolanaError) -> ToolError {
    ToolError::Failed {
        name: "solana".to_string(),
        message: e.to_string(),
    }
}

pub struct BalanceTool {
    client: Arc<SolanaClient>,
}

#[async_trait]
impl Tool for BalanceTool {
    fn name(&self) -> &str {
        "solana_get_balance"
    }
    fn description(&self) -> &str {
        "Get the SOL balance (lamports) of a Solana address"
    }
    fn input_schema(&self) -> Value {
        json!({"type": "object", "properties": {"pubkey": {"type": "string"}}, "required": ["pubkey"]})
    }
    async fn call(&self, arguments: Value) -> Result<Value, ToolError> {
        let pubkey = parse_pubkey(&arguments, "pubkey")?;
        let lamports = self.client.balance(&pubkey).await.map_err(rpc_tool_err)?;
        Ok(json!({"pubkey": pubkey.to_string(), "lamports": lamports, "sol": lamports as f64 / 1e9}))
    }
}

pub struct TokenAccountsTool {
    client: Arc<SolanaClient>,
}

#[async_trait]
impl Tool for TokenAccountsTool {
    fn name(&self) -> &str {
        "solana_get_token_accounts"
    }
    fn description(&self) -> &str {
        "List SPL token accounts (mint, amount) owned by a Solana address"
    }
    fn input_schema(&self) -> Value {
        json!({"type": "object", "properties": {"owner": {"type": "string"}}, "required": ["owner"]})
    }
    async fn call(&self, arguments: Value) -> Result<Value, ToolError> {
        let owner = parse_pubkey(&arguments, "owner")?;
        let accounts = self.client.token_accounts(&owner).await.map_err(rpc_tool_err)?;
        let list: Vec<Value> = accounts
            .iter()
            .map(|a| json!({"account": a.account, "mint": a.mint, "amount": a.amount, "ui_amount": a.ui_amount}))
            .collect();
        Ok(json!({"owner": owner.to_string(), "token_accounts": list}))
    }
}

pub struct TxStatusTool {
    client: Arc<SolanaClient>,
}

#[async_trait]
impl Tool for TxStatusTool {
    fn name(&self) -> &str {
        "solana_get_tx_status"
    }
    fn description(&self) -> &str {
        "Get the status of a transaction by signature"
    }
    fn input_schema(&self) -> Value {
        json!({"type": "object", "properties": {"signature": {"type": "string"}}, "required": ["signature"]})
    }
    async fn call(&self, arguments: Value) -> Result<Value, ToolError> {
        let sig_str = arguments["signature"].as_str().ok_or_else(|| ToolError::Failed {
            name: "solana".to_string(),
            message: "missing string argument `signature`".to_string(),
        })?;
        let signature: solana_sdk::signature::Signature =
            sig_str.parse().map_err(|e| ToolError::Failed {
                name: "solana".to_string(),
                message: format!("invalid signature: {e}"),
            })?;
        match self.client.signature_status(&signature).await.map_err(rpc_tool_err)? {
            Some(status) => Ok(json!({
                "signature": sig_str,
                "found": true,
                "slot": status.slot,
                "confirmations": status.confirmations,
                "err": status.err,
                "confirmation_status": status.confirmation_status,
            })),
            None => Ok(json!({"signature": sig_str, "found": false})),
        }
    }
}

/// Consequential: builds, signs (via the provided signer), and broadcasts
/// a SOL transfer. Only reachable in the loop after the Jev gate proceeds
/// and dry-run is off.
pub struct TransferTool {
    client: Arc<SolanaClient>,
    signer: Arc<dyn SolanaSigner>,
}

#[async_trait]
impl Tool for TransferTool {
    fn name(&self) -> &str {
        "solana_transfer"
    }
    fn description(&self) -> &str {
        "Transfer SOL from the signer's wallet to another address (consequential)"
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "to": {"type": "string"},
                "lamports": {"type": "integer", "minimum": 1}
            },
            "required": ["to", "lamports"]
        })
    }
    fn consequential(&self) -> bool {
        true
    }
    async fn call(&self, arguments: Value) -> Result<Value, ToolError> {
        let to = parse_pubkey(&arguments, "to")?;
        let lamports = arguments["lamports"].as_u64().ok_or_else(|| ToolError::Failed {
            name: "solana".to_string(),
            message: "missing integer argument `lamports`".to_string(),
        })?;
        let blockhash = self.client.latest_blockhash().await.map_err(rpc_tool_err)?;
        let tx = build_transfer(&self.signer.pubkey(), &to, lamports, blockhash);
        let signed = self.signer.sign_transaction(tx).await.map_err(rpc_tool_err)?;
        let signature = self.client.send_transaction(&signed).await.map_err(rpc_tool_err)?;
        Ok(json!({"signature": signature.to_string(), "to": to.to_string(), "lamports": lamports}))
    }
}

/// Register the read tools, plus the transfer tool when a signer is given.
pub fn register_solana_tools(
    registry: &mut ToolRegistry,
    client: Arc<SolanaClient>,
    signer: Option<Arc<dyn SolanaSigner>>,
) {
    registry.register(Arc::new(BalanceTool {
        client: client.clone(),
    }));
    registry.register(Arc::new(TokenAccountsTool {
        client: client.clone(),
    }));
    registry.register(Arc::new(TxStatusTool {
        client: client.clone(),
    }));
    if let Some(signer) = signer {
        registry.register(Arc::new(TransferTool { client, signer }));
    }
}
