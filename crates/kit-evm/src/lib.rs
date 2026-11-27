//! kit-evm — the OpenClawd kit's EVM + cross-chain rails, ported forward.
//!
//! The original kit carried EVM tools on alloy 0.9 and a LiFi cross-chain
//! client. This crate keeps that scope on **current alloy (1.x)**:
//!
//! - [`EvmClient`] — read balances and build (unsigned) transfer requests
//!   over an HTTP JSON-RPC provider.
//! - [`lifi::LiFiClient`] — LiFi chains / tokens / quote reads for
//!   cross-chain routing.
//! - [`signer::LocalEvmSigner`] — the original kit's local-signer custody
//!   (env private key), kept for parity; the kit's *default* flows build
//!   unsigned requests for wallet / remote signing.
//!
//! This crate is in the workspace but not in `default-members`: build it
//! with `cargo check -p kit-evm`.

pub mod lifi;
pub mod signer;

use alloy::network::TransactionBuilder;
use alloy::primitives::{Address, U256};
use alloy::providers::{Provider, ProviderBuilder};
use alloy::rpc::types::TransactionRequest;
use serde_json::{json, Value};

#[derive(Debug, thiserror::Error)]
pub enum EvmError {
    #[error("EVM RPC error: {0}")]
    Rpc(String),
    #[error("invalid address: {0}")]
    Address(String),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}

pub struct EvmClient {
    rpc_url: String,
}

impl EvmClient {
    pub fn new(rpc_url: impl Into<String>) -> Self {
        Self {
            rpc_url: rpc_url.into(),
        }
    }

    fn provider(&self) -> Result<impl Provider, EvmError> {
        let url = self
            .rpc_url
            .parse()
            .map_err(|e| EvmError::Rpc(format!("bad RPC URL: {e}")))?;
        Ok(ProviderBuilder::new().connect_http(url))
    }

    pub async fn balance(&self, address: &str) -> Result<U256, EvmError> {
        let addr: Address = address
            .parse()
            .map_err(|e| EvmError::Address(format!("{e}")))?;
        let provider = self.provider()?;
        provider
            .get_balance(addr)
            .await
            .map_err(|e| EvmError::Rpc(e.to_string()))
    }

    pub async fn chain_id(&self) -> Result<u64, EvmError> {
        let provider = self.provider()?;
        provider
            .get_chain_id()
            .await
            .map_err(|e| EvmError::Rpc(e.to_string()))
    }

    /// Build an unsigned transfer request as JSON (for wallet / remote
    /// signing). The kit does not broadcast from this path.
    pub fn build_transfer_request(from: &str, to: &str, value_wei: U256) -> Result<Value, EvmError> {
        let from_addr: Address = from.parse().map_err(|e| EvmError::Address(format!("{e}")))?;
        let to_addr: Address = to.parse().map_err(|e| EvmError::Address(format!("{e}")))?;
        let request = TransactionRequest::default()
            .with_from(from_addr)
            .with_to(to_addr)
            .with_value(value_wei);
        Ok(json!({
            "from": format!("{from_addr}"),
            "to": format!("{to_addr}"),
            "value": value_wei.to_string(),
            "request": serde_json::to_value(&request).unwrap_or(Value::Null),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfer_request_builds() {
        let v = EvmClient::build_transfer_request(
            "0x0000000000000000000000000000000000000001",
            "0x0000000000000000000000000000000000000002",
            U256::from(1000u64),
        )
        .unwrap();
        assert_eq!(v["value"], "1000");
    }

    #[test]
    fn bad_address_rejected() {
        assert!(EvmClient::build_transfer_request("nope", "0x0000000000000000000000000000000000000002", U256::ZERO).is_err());
    }
}
