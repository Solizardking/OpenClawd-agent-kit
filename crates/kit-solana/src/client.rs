//! Solana reads over the modern split-crate RPC client.

use serde::Deserialize;
use serde_json::Value;
use solana_account_decoder::UiAccountData;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client_api::request::TokenAccountsFilter;
use solana_sdk::hash::Hash;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_sdk::transaction::Transaction;

use crate::SolanaError;

pub const TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

#[derive(Debug, Clone, PartialEq)]
pub struct TokenAccount {
    /// The token account's own address.
    pub account: String,
    pub mint: String,
    /// Raw amount in the mint's smallest unit.
    pub amount: u64,
    /// UI amount string as reported by the RPC (decimal-adjusted).
    pub ui_amount: String,
}

/// A transaction's status, flattened so callers don't depend on the RPC
/// client's internal status types.
#[derive(Debug, Clone, PartialEq)]
pub struct TxStatus {
    pub slot: u64,
    pub confirmations: Option<u64>,
    pub err: Option<String>,
    pub confirmation_status: Option<String>,
}

pub struct SolanaClient {
    rpc: RpcClient,
    url: String,
}

impl SolanaClient {
    pub fn new(url: impl Into<String>) -> Self {
        let url = url.into();
        Self {
            rpc: RpcClient::new(url.clone()),
            url,
        }
    }

    /// From `SOLANA_RPC_URL`, defaulting to the public mainnet endpoint.
    /// (Fleet rule: prefer a private endpoint — Helius/QuikNode — in
    /// production; the public endpoint is a correctness fallback only.)
    pub fn from_env() -> Self {
        let url = std::env::var("SOLANA_RPC_URL")
            .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
        Self::new(url)
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    fn rpc_err(e: impl std::fmt::Display) -> SolanaError {
        SolanaError::Rpc(e.to_string())
    }

    pub async fn balance(&self, pubkey: &Pubkey) -> Result<u64, SolanaError> {
        self.rpc.get_balance(pubkey).await.map_err(Self::rpc_err)
    }

    pub async fn latest_blockhash(&self) -> Result<Hash, SolanaError> {
        self.rpc
            .get_latest_blockhash()
            .await
            .map_err(Self::rpc_err)
    }

    pub async fn slot(&self) -> Result<u64, SolanaError> {
        self.rpc.get_slot().await.map_err(Self::rpc_err)
    }

    pub async fn get_account(
        &self,
        pubkey: &Pubkey,
    ) -> Result<solana_sdk::account::Account, SolanaError> {
        self.rpc.get_account(pubkey).await.map_err(Self::rpc_err)
    }

    /// All SPL token accounts owned by `owner` (legacy Token program),
    /// parsed from the RPC's jsonParsed encoding.
    pub async fn token_accounts(&self, owner: &Pubkey) -> Result<Vec<TokenAccount>, SolanaError> {
        let program: Pubkey = TOKEN_PROGRAM_ID
            .parse()
            .expect("token program id is a valid pubkey");
        let accounts = self
            .rpc
            .get_token_accounts_by_owner(owner, TokenAccountsFilter::ProgramId(program))
            .await
            .map_err(Self::rpc_err)?;

        let mut out = Vec::new();
        for keyed in accounts {
            if let UiAccountData::Json(parsed) = keyed.account.data {
                let info = &parsed.parsed["info"];
                let amount = info["tokenAmount"]["amount"]
                    .as_str()
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0);
                let ui_amount = info["tokenAmount"]["uiAmountString"]
                    .as_str()
                    .unwrap_or("0")
                    .to_string();
                out.push(TokenAccount {
                    account: keyed.pubkey,
                    mint: info["mint"].as_str().unwrap_or_default().to_string(),
                    amount,
                    ui_amount,
                });
            }
        }
        Ok(out)
    }

    pub async fn signature_status(
        &self,
        signature: &Signature,
    ) -> Result<Option<TxStatus>, SolanaError> {
        let response = self
            .rpc
            .get_signature_statuses(&[*signature])
            .await
            .map_err(Self::rpc_err)?;
        let Some(status) = response.value.into_iter().next().flatten() else {
            return Ok(None);
        };
        // Convert through JSON so this crate doesn't pin the RPC client's
        // internal status types.
        #[derive(Deserialize)]
        struct StatusWire {
            slot: u64,
            confirmations: Option<u64>,
            err: Option<Value>,
            #[serde(rename = "confirmationStatus")]
            confirmation_status: Option<String>,
        }
        let wire: StatusWire = serde_json::from_value(
            serde_json::to_value(&status).map_err(|e| SolanaError::Rpc(e.to_string()))?,
        )
        .map_err(|e| SolanaError::Rpc(e.to_string()))?;
        Ok(Some(TxStatus {
            slot: wire.slot,
            confirmations: wire.confirmations,
            err: wire.err.map(|e| e.to_string()),
            confirmation_status: wire.confirmation_status,
        }))
    }

    /// Broadcast a fully-signed transaction. This is the *only* send path
    /// in the crate, and the tools layer only reaches it for a transaction
    /// that a signer produced after the Jev gate cleared the call.
    pub async fn send_transaction(&self, tx: &Transaction) -> Result<Signature, SolanaError> {
        self.rpc.send_transaction(tx).await.map_err(Self::rpc_err)
    }
}
