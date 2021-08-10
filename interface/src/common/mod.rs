use self::ss58format::{Ss58AddressFormat, Ss58Codec};
use parity_scale_codec::{Decode, Encode};

pub mod ss58format;
/// Re-export of the [`parity-scale-codec`](https://crates.io/crates/parity-scale-codec) crate.
pub mod scale {
    pub use parity_scale_codec::*;
}

pub type Balance = u128;
pub type Sr25519 = sp_core::sr25519::Pair;
pub type Ed25519 = sp_core::ed25519::Pair;
pub type Ecdsa = sp_core::ecdsa::Pair;

#[derive(Debug, Clone, Copy)]
pub enum Network {
    Polkadot,
    Kusama,
    Westend,
    Custom([u8; 32]),
}

impl Network {
    pub fn genesis(&self) -> [u8; 32] {
        let mut genesis = [0; 32];

        let hash_str = match self {
            Self::Polkadot => "c0096358534ec8d21d01d34b836eed476a1c343f8724fa2153dc0725ad797a90",
            Self::Kusama => "cd9b8e2fc2f57c4570a86319b005832080e0c478ab41ae5d44e23705872f5ad3",
            Self::Westend => "44ef51c86927a1e2da55754dba9684dd6ff9bac8c61624ffe958be656c42e036",
            Self::Custom(genesis) => return *genesis,
        };

        hex::decode_to_slice(hash_str, &mut genesis).unwrap();
        genesis
    }
}

pub struct KeyPairBuilder<T>(std::marker::PhantomData<T>);

impl<T: sp_core::crypto::Pair> KeyPairBuilder<T> {
    pub fn generate() -> (T, T::Seed) {
        T::generate()
    }
    pub fn from_seed(seed: &T::Seed) -> T {
        T::from_seed(seed)
    }
    pub fn from_phase(
        phase: &str,
        password: Option<&str>,
    ) -> Result<(T, T::Seed), sp_core::crypto::SecretStringError> {
        T::from_phrase(phase, password)
    }
}

#[derive(Clone)]
pub enum MultiKeyPair {
    Ed25519(Ed25519),
    Sr25519(Sr25519),
    Ecdsa(Ecdsa),
}

impl From<Ed25519> for MultiKeyPair {
    fn from(val: Ed25519) -> Self {
        MultiKeyPair::Ed25519(val)
    }
}

impl From<Sr25519> for MultiKeyPair {
    fn from(val: Sr25519) -> Self {
        MultiKeyPair::Sr25519(val)
    }
}

impl From<Ecdsa> for MultiKeyPair {
    fn from(val: Ecdsa) -> Self {
        MultiKeyPair::Ecdsa(val)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
// TODO: Custom Encode/Decode implementation. See https://substrate.dev/rustdocs/latest/sp_runtime/generic/enum.Era.html
pub enum Mortality {
    Immortal,
    Mortal([u8; 32]),
}

impl Mortality {
    /// The block number from where the period of mortality begins. The
    /// corresponding block hash required for the final transaction must be
    /// retrieved from the blockchain manually.
    pub fn mortal(current: u64, period: u64, phase: u64) -> u64 {
        (current.max(phase) - phase) / period * period + phase
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub enum MultiSignature {
    Ed25519(sp_core::ed25519::Signature),
    Sr25519(sp_core::sr25519::Signature),
    Ecdsa(sp_core::ecdsa::Signature),
}

impl From<sp_core::ed25519::Signature> for MultiSignature {
    fn from(val: sp_core::ed25519::Signature) -> Self {
        MultiSignature::Ed25519(val)
    }
}

impl From<sp_core::sr25519::Signature> for MultiSignature {
    fn from(val: sp_core::sr25519::Signature) -> Self {
        MultiSignature::Sr25519(val)
    }
}

impl From<sp_core::ecdsa::Signature> for MultiSignature {
    fn from(val: sp_core::ecdsa::Signature) -> Self {
        MultiSignature::Ecdsa(val)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct AccountId32([u8; 32]);

impl AccountId32 {
    pub fn to_ss58_address(&self, format: Ss58AddressFormat) -> String {
        Ss58Codec::to_string_with_format(&self.0, format)
    }
    /// Returns the underlying public key or the blake2b hash in case of ECDSA.
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }
    // TODO: Add method to extra public key.
}

impl From<MultiKeyPair> for AccountId32 {
    fn from(_: MultiKeyPair) -> Self {
        unimplemented!()
    }
}
