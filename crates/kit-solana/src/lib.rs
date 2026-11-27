//! kit-solana — Solana tools for the OpenClawd agent kit, on the modern
//! split-crate SDK (solana-sdk / solana-rpc-client 3.x).
//!
//! - [`client`] — balances, token accounts, tx status, blockhash, send.
//! - [`transfer`] — unsigned transfer building + base64 wire format.
//! - [`signer`] — signer abstraction + task-local [`signer::SignerContext`];
//!   default flows are unsigned / remote-signed (no key custody).
//! - [`verify`] — the [`verify::StateProof`] / [`verify::LightVerify`]
//!   seam where a Solana Lean client plugs in (see `docs/ohr.md`).
//! - [`tools`] — the same capabilities as kit [`kit_core::Tool`]s.

pub mod client;
pub mod signer;
pub mod tools;
pub mod transfer;
pub mod verify;

pub use client::{SolanaClient, TokenAccount, TxStatus};
pub use signer::{RemoteSigner, SignerContext, SolanaSigner, UnsignedSigner};
pub use verify::{AccountProof, LightVerify, RpcStateProof, StateProof};

#[cfg(feature = "local-signer")]
pub use signer::LocalKeypairSigner;

// Re-exported so downstream crates can name the core SDK types without
// taking their own solana-sdk dependency.
pub use solana_sdk::hash::Hash;
pub use solana_sdk::pubkey::Pubkey;
pub use solana_sdk::signature::Signature;
pub use solana_sdk::transaction::Transaction;

#[derive(Debug, thiserror::Error)]
pub enum SolanaError {
    #[error("Solana RPC error: {0}")]
    Rpc(String),
    #[error("transaction encoding error: {0}")]
    Encoding(String),
    #[error("signing error: {0}")]
    Signing(String),
}
