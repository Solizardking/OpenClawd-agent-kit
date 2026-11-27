//! Transfer building + the transaction wire format.
//!
//! The kit builds **unsigned** transactions. Signing is a separate step
//! owned by a [`crate::signer::SolanaSigner`] — by default an
//! [`crate::signer::UnsignedSigner`] or a remote signer, so the kit never
//! needs key custody to construct a transfer.

use base64::Engine;
use solana_sdk::hash::Hash;
use solana_sdk::pubkey::Pubkey;
use solana_system_interface::instruction as system_instruction;
use solana_sdk::transaction::Transaction;

use crate::SolanaError;

/// Build an unsigned SOL transfer transaction.
pub fn build_transfer(payer: &Pubkey, to: &Pubkey, lamports: u64, blockhash: Hash) -> Transaction {
    let ix = system_instruction::transfer(payer, to, lamports);
    let mut tx = Transaction::new_with_payer(&[ix], Some(payer));
    tx.message.recent_blockhash = blockhash;
    tx
}

/// Serialize a transaction to the canonical Solana wire format, base64.
/// This is the format wallets and remote signers expect.
pub fn encode_tx(tx: &Transaction) -> Result<String, SolanaError> {
    let bytes = bincode::serde::encode_to_vec(tx, bincode::config::standard())
        .map_err(|e| SolanaError::Encoding(e.to_string()))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
}

/// Inverse of [`encode_tx`].
pub fn decode_tx(encoded: &str) -> Result<Transaction, SolanaError> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|e| SolanaError::Encoding(e.to_string()))?;
    let (tx, _consumed) =
        bincode::serde::decode_from_slice::<Transaction, _>(&bytes, bincode::config::standard())
            .map_err(|e| SolanaError::Encoding(e.to_string()))?;
    Ok(tx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfer_builds_and_round_trips() {
        let payer = Pubkey::new_unique();
        let to = Pubkey::new_unique();
        let tx = build_transfer(&payer, &to, 12_345, Hash::new_unique());
        assert_eq!(tx.message.account_keys[0], payer);
        assert_eq!(tx.signatures.len(), 1);
        let encoded = encode_tx(&tx).expect("encode");
        let decoded = decode_tx(&encoded).expect("decode");
        assert_eq!(decoded.message, tx.message);
    }
}
