//! EVM signer context, carried from the original kit.
//!
//! The default is [`UnsignedEvmSigner`]: it knows an address and builds
//! requests, but holds no key and cannot sign — signing belongs to the
//! operator's wallet or a remote signer. [`LocalEvmSigner`] preserves the
//! original kit's env-key custody for parity; it is never constructed
//! implicitly — a host must call [`LocalEvmSigner::from_env`] on purpose.

use alloy::signers::local::PrivateKeySigner;

use crate::EvmError;

pub struct UnsignedEvmSigner {
    address: String,
}

impl UnsignedEvmSigner {
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            address: address.into(),
        }
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    /// Always an error: this signer holds no key, by design.
    pub fn sign(&self, _payload: &[u8]) -> Result<Vec<u8>, EvmError> {
        Err(EvmError::Rpc(
            "unsigned signer: no key material held; route signing to a wallet or remote signer"
                .to_string(),
        ))
    }
}

pub struct LocalEvmSigner {
    signer: PrivateKeySigner,
}

impl LocalEvmSigner {
    /// Read `ETHEREUM_PRIVATE_KEY` from the environment. Explicit opt-in
    /// custody, identical in spirit to the original kit.
    pub fn from_env() -> Result<Self, EvmError> {
        let key = std::env::var("ETHEREUM_PRIVATE_KEY")
            .map_err(|_| EvmError::Rpc("ETHEREUM_PRIVATE_KEY is not set".to_string()))?;
        let signer: PrivateKeySigner = key
            .parse()
            .map_err(|e| EvmError::Rpc(format!("invalid ETHEREUM_PRIVATE_KEY: {e}")))?;
        Ok(Self { signer })
    }

    pub fn address(&self) -> String {
        format!("{}", self.signer.address())
    }
}
