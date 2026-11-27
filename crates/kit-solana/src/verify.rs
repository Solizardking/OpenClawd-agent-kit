//! The state-proof seam: where a Solana **Lean client** plugs in.
//!
//! An agent that reads chain state through a plain RPC endpoint is
//! trusting that endpoint. A light / Lean client instead *verifies* state
//! against validator-voted roots — and the Solana Foundation's solanalib
//! now models the runtime those proofs are about, in Lean.
//!
//! This module defines the seam as traits, with an honest RPC-backed
//! implementation:
//!
//! - [`StateProof`] produces and checks an [`AccountProof`] — a digest of
//!   an account's state at a slot.
//! - [`LightVerify`] is the verification interface a Lean-client backend
//!   implements. [`RpcStateProof`]'s implementation re-reads the account
//!   through RPC and compares digests: that detects drift, but it is
//!   **not** cryptographic inclusion verification and must not be
//!   described as such. A real backend (see `docs/ohr.md`) replaces the
//!   internals without changing call sites.
//!
//! Keeping the trait surface stable is the point: when the dev supplies
//! the Lean client, it lands behind [`LightVerify`] and every consumer of
//! verified state in the kit upgrades at once.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use solana_sdk::hash::Hash;
use solana_sdk::pubkey::Pubkey;

use crate::client::SolanaClient;
use crate::SolanaError;

/// A digest of one account's state at one slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountProof {
    pub pubkey: String,
    pub slot: u64,
    pub lamports: u64,
    pub owner: String,
    pub executable: bool,
    /// Hash of the account's data bytes (SHA-256 via the SDK's `Hash`).
    pub data_hash: String,
}

#[async_trait]
pub trait StateProof: Send + Sync {
    async fn prove_account(&self, pubkey: &Pubkey) -> Result<AccountProof, SolanaError>;
    async fn verify(&self, proof: &AccountProof) -> Result<bool, SolanaError>;
}

/// The Lean-client seam. See the module docs.
#[async_trait]
pub trait LightVerify: Send + Sync {
    /// Verify that `expected` describes the account's actual state.
    /// A Lean-client backend checks this against a verified state root;
    /// the RPC stub re-reads and compares.
    async fn verify_account_state(
        &self,
        pubkey: &Pubkey,
        expected: &AccountProof,
    ) -> Result<bool, SolanaError>;
}

/// RPC-backed [`StateProof`]: reads through a normal RPC endpoint.
/// Honest label: this is an *attestation digest*, not a light-client proof.
pub struct RpcStateProof {
    client: SolanaClient,
}

impl RpcStateProof {
    pub fn new(client: SolanaClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl StateProof for RpcStateProof {
    async fn prove_account(&self, pubkey: &Pubkey) -> Result<AccountProof, SolanaError> {
        let account = self
            .client
            .get_account(pubkey)
            .await?;
        let slot = self.client.slot().await?;
        Ok(AccountProof {
            pubkey: pubkey.to_string(),
            slot,
            lamports: account.lamports,
            owner: account.owner.to_string(),
            executable: account.executable,
            data_hash: Hash::new(&account.data).to_string(),
        })
    }

    async fn verify(&self, proof: &AccountProof) -> Result<bool, SolanaError> {
        let pubkey: Pubkey = proof
            .pubkey
            .parse()
            .map_err(|e| SolanaError::Rpc(format!("bad pubkey in proof: {e}")))?;
        let current = self.prove_account(&pubkey).await?;
        // Slot is excluded: state may legitimately be re-read at a later
        // slot. Everything that *is* the state must match.
        Ok(current.lamports == proof.lamports
            && current.owner == proof.owner
            && current.executable == proof.executable
            && current.data_hash == proof.data_hash)
    }
}

#[async_trait]
impl LightVerify for RpcStateProof {
    async fn verify_account_state(
        &self,
        _pubkey: &Pubkey,
        expected: &AccountProof,
    ) -> Result<bool, SolanaError> {
        self.verify(expected).await
    }
}
