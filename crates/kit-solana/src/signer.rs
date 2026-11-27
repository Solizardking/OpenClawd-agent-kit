//! Signer abstraction + signer context, carried forward from the original
//! kit's `signer` module and re-based on the modern SDK.
//!
//! Custody rules (fleet standing policy):
//! - The **default** flows are unsigned / remote-signed. The kit builds
//!   transactions; something the operator controls signs them.
//! - [`UnsignedSigner`] holds only a pubkey and cannot sign — it exists so
//!   read + build flows have a signer-shaped identity.
//! - [`RemoteSigner`] delegates to an operator-run signing endpoint using
//!   the kit's simple base64 JSON protocol.
//! - [`LocalKeypairSigner`] (feature `local-signer`) preserves the original
//!   kit's env-keypair custody for parity. It is never constructed
//!   implicitly.

use std::future::Future;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_sdk::transaction::Transaction;

use crate::transfer::{decode_tx, encode_tx};
use crate::SolanaError;

#[async_trait]
pub trait SolanaSigner: Send + Sync {
    fn pubkey(&self) -> Pubkey;

    /// Sign a built transaction. Implementations may return it unchanged
    /// (unsigned signer), partially signed, or fully signed.
    async fn sign_transaction(&self, tx: Transaction) -> Result<Transaction, SolanaError>;

    async fn sign_message(&self, message: &[u8]) -> Result<Signature, SolanaError>;
}

/// Pubkey-only identity. Cannot sign anything.
pub struct UnsignedSigner {
    pubkey: Pubkey,
}

impl UnsignedSigner {
    pub fn new(pubkey: Pubkey) -> Self {
        Self { pubkey }
    }
}

#[async_trait]
impl SolanaSigner for UnsignedSigner {
    fn pubkey(&self) -> Pubkey {
        self.pubkey
    }

    async fn sign_transaction(&self, tx: Transaction) -> Result<Transaction, SolanaError> {
        // Deliberately returns the transaction unsigned: callers that need
        // a signature must use a real signer. Sending this transaction
        // will fail signature verification at the cluster — fail-closed.
        Ok(tx)
    }

    async fn sign_message(&self, _message: &[u8]) -> Result<Signature, SolanaError> {
        Err(SolanaError::Signing(
            "unsigned signer holds no key; route signing to a wallet or remote signer".to_string(),
        ))
    }
}

/// A signer behind an operator-run HTTP endpoint.
///
/// Protocol (the kit's convention): `POST {endpoint}` with
/// `{"transaction": "<base64 wire format>"}` → `{"transaction": "<base64>"}`
/// for transactions, and `{"message": "<base64>"}` → `{"signature": "<bs58>"}`
/// for messages. The endpoint authenticates and authorizes on its side;
/// the kit sends no credentials beyond an optional bearer token from
/// `KIT_REMOTE_SIGNER_TOKEN`.
pub struct RemoteSigner {
    pubkey: Pubkey,
    endpoint: String,
    http: reqwest::Client,
}

impl RemoteSigner {
    pub fn new(pubkey: Pubkey, endpoint: impl Into<String>) -> Self {
        Self {
            pubkey,
            endpoint: endpoint.into(),
            http: reqwest::Client::new(),
        }
    }

    async fn post(&self, body: serde_json::Value) -> Result<serde_json::Value, SolanaError> {
        let mut req = self.http.post(&self.endpoint).json(&body);
        if let Ok(token) = std::env::var("KIT_REMOTE_SIGNER_TOKEN") {
            req = req.bearer_auth(token);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| SolanaError::Signing(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(SolanaError::Signing(format!(
                "remote signer returned HTTP {}",
                resp.status()
            )));
        }
        resp.json()
            .await
            .map_err(|e| SolanaError::Signing(e.to_string()))
    }
}

#[async_trait]
impl SolanaSigner for RemoteSigner {
    fn pubkey(&self) -> Pubkey {
        self.pubkey
    }

    async fn sign_transaction(&self, tx: Transaction) -> Result<Transaction, SolanaError> {
        let encoded = encode_tx(&tx)?;
        let resp = self.post(json!({"transaction": encoded})).await?;
        let signed = resp["transaction"]
            .as_str()
            .ok_or_else(|| SolanaError::Signing("remote signer response missing `transaction`".to_string()))?;
        decode_tx(signed)
    }

    async fn sign_message(&self, message: &[u8]) -> Result<Signature, SolanaError> {
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(message);
        let resp = self.post(json!({"message": encoded})).await?;
        let sig = resp["signature"]
            .as_str()
            .ok_or_else(|| SolanaError::Signing("remote signer response missing `signature`".to_string()))?;
        sig.parse()
            .map_err(|e| SolanaError::Signing(format!("bad signature from remote signer: {e}")))
    }
}

/// The original kit's local-keypair custody, feature-gated for parity.
/// Constructed only by an explicit host call — never implicitly.
#[cfg(feature = "local-signer")]
pub struct LocalKeypairSigner {
    keypair: Arc<solana_sdk::signature::Keypair>,
}

#[cfg(feature = "local-signer")]
impl LocalKeypairSigner {
    /// From a base58 secret key (the original kit's `SOLANA_PRIVATE_KEY`
    /// flow). The host decides where the string comes from; this crate
    /// never reads key material from disk or env on its own.
    pub fn from_base58(secret: &str) -> Self {
        Self {
            keypair: Arc::new(solana_sdk::signature::Keypair::from_base58_string(secret)),
        }
    }
}

#[cfg(feature = "local-signer")]
#[async_trait]
impl SolanaSigner for LocalKeypairSigner {
    fn pubkey(&self) -> Pubkey {
        use solana_sdk::signer::Signer as _;
        self.keypair.pubkey()
    }

    async fn sign_transaction(&self, mut tx: Transaction) -> Result<Transaction, SolanaError> {
        let blockhash = tx.message.recent_blockhash;
        tx.try_sign(&[&*self.keypair], blockhash)
            .map_err(|e| SolanaError::Signing(e.to_string()))?;
        Ok(tx)
    }

    async fn sign_message(&self, message: &[u8]) -> Result<Signature, SolanaError> {
        use solana_sdk::signer::Signer as _;
        Ok(self.keypair.sign_message(message))
    }
}

tokio::task_local! {
    static CURRENT_SIGNER: Arc<dyn SolanaSigner>;
}

/// Task-local signer context — same shape as the original kit's
/// `SignerContext`: hosts install a signer for the duration of a task and
/// tools retrieve it without threading it through every call.
pub struct SignerContext;

impl SignerContext {
    pub async fn with_signer<T>(
        signer: Arc<dyn SolanaSigner>,
        f: impl Future<Output = Result<T, SolanaError>> + Send,
    ) -> Result<T, SolanaError> {
        CURRENT_SIGNER.scope(signer, f).await
    }

    /// The signer installed for this task. Panics outside a
    /// [`SignerContext::with_signer`] scope — same contract as the
    /// original kit; a missing signer is a programming error, and any
    /// default would be a custody decision made silently.
    pub async fn current() -> Arc<dyn SolanaSigner> {
        CURRENT_SIGNER.get().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unsigned_signer_cannot_sign_messages() {
        let signer = UnsignedSigner::new(Pubkey::new_unique());
        assert!(signer.sign_message(b"hello").await.is_err());
    }

    #[tokio::test]
    async fn signer_context_scopes() {
        let signer: Arc<dyn SolanaSigner> = Arc::new(UnsignedSigner::new(Pubkey::new_unique()));
        let expected = signer.pubkey();
        let got = SignerContext::with_signer(signer, async {
            Ok(SignerContext::current().await.pubkey())
        })
        .await
        .unwrap();
        assert_eq!(got, expected);
    }
}
