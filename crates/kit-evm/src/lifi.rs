//! LiFi cross-chain client (read paths): chains, tokens, and quotes.
//!
//! Ported from the original kit's `cross_chain/lifi` module to plain
//! reqwest + serde_json. Quote responses are passed through as JSON: the
//! LiFi schema is broad and evolves; the kit treats it as data for the
//! agent, and any *execution* of a quote goes through the Jev gate and a
//! signer the operator chose — never through this client.

use serde_json::Value;

use crate::EvmError;

pub const LIFI_API: &str = "https://li.quest/v1";

pub struct LiFiClient {
    http: reqwest::Client,
    base: String,
}

impl Default for LiFiClient {
    fn default() -> Self {
        Self::new()
    }
}

impl LiFiClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
            base: LIFI_API.to_string(),
        }
    }

    async fn get(&self, path: &str) -> Result<Value, EvmError> {
        let resp = self
            .http
            .get(format!("{}{path}", self.base))
            .send()
            .await?;
        let status = resp.status();
        let value: Value = resp.json().await?;
        if !status.is_success() {
            return Err(EvmError::Rpc(format!("LiFi HTTP {status}: {value}")));
        }
        Ok(value)
    }

    pub async fn chains(&self) -> Result<Value, EvmError> {
        self.get("/chains").await
    }

    pub async fn tokens(&self, chain: u64) -> Result<Value, EvmError> {
        self.get(&format!("/tokens?chain={chain}")).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn quote(
        &self,
        from_chain: u64,
        to_chain: u64,
        from_token: &str,
        to_token: &str,
        from_amount: &str,
        from_address: &str,
    ) -> Result<Value, EvmError> {
        self.get(&format!(
            "/quote?fromChain={from_chain}&toChain={to_chain}&fromToken={from_token}&toToken={to_token}&fromAmount={from_amount}&fromAddress={from_address}"
        ))
        .await
    }
}
