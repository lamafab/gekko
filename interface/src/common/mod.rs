use self::ss58format::{Ss58AddressFormat, Ss58Codec};
use crate::blake2b;
use ed25519_dalek::Keypair as EdKeypair;
use parity_scale_codec::{Decode, Encode};
use schnorrkel::keys::Keypair as SrKeypair;
use secp256k1::{Secp256k1, SecretKey};

pub mod ss58format;

/// Re-export of the [`parity-scale-codec`](https://crates.io/crates/parity-scale-codec) crate.
pub mod scale {
    pub use parity_scale_codec::*;
    pub mod crypto {
        pub use ed25519_dalek as ed25519;
        pub use schnorrkel as sr25519;
        pub use secp256k1;
    }
}

pub type Balance = u128;

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
pub enum MultiAddress<AccountId, AccountIndex> {
    Id(AccountId),
    Index(#[codec(compact)] AccountIndex),
    Raw(Vec<u8>),
    Address32([u8; 32]),
    Address20([u8; 20]),
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub enum MultiSignature {
    Ed25519([u8; 64]),
    Sr25519([u8; 64]),
    Ecdsa([u8; 65]),
}

impl From<MultiSigner> for MultiAddress<AccountId32, ()> {
    fn from(_signer: MultiSigner) -> Self {
        unimplemented!()
    }
}

#[derive(Debug)]
pub enum MultiSigner {
    Ed25519(EdKeypair),
    Sr25519(SrKeypair),
    Ecdsa(SecretKey),
}

impl MultiSigner {
    pub fn to_public_key(&self) -> Vec<u8> {
        // This method returns a vector rather than an array since public key
        // sizes vary in size.
        match self {
            Self::Ed25519(pair) => pair.public.to_bytes().to_vec(),
            Self::Sr25519(pair) => pair.public.to_bytes().to_vec(),
            Self::Ecdsa(sec_key) => {
                secp256k1::key::PublicKey::from_secret_key(&Secp256k1::signing_only(), &sec_key)
                    .serialize()
                    .to_vec()
            }
        }
    }
    pub fn to_account_id(&self) -> AccountId32 {
        let pub_key = match self {
            Self::Ed25519(pair) => pair.public.to_bytes(),
            Self::Sr25519(pair) => pair.public.to_bytes(),
            Self::Ecdsa(sec_key) => {
                let pub_key = secp256k1::key::PublicKey::from_secret_key(
                    &Secp256k1::signing_only(),
                    &sec_key,
                )
                .serialize();

                // Hashed, since the ECDSA public key is 33 bytes.
                blake2b(&pub_key)
            }
        };

        AccountId32(pub_key)
    }
    pub fn to_ss58_address(&self, format: Ss58AddressFormat) -> String {
        Ss58Codec::to_string_with_format(&self.to_account_id().0, format)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct AccountId32([u8; 32]);
