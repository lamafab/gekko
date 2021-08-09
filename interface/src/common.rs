use ed25519_dalek::Keypair as EdKeypair;
use parity_scale_codec::{Decode, Encode};
use schnorrkel::keys::Keypair as SrKeypair;
use secp256k1::SecretKey;

/// Re-export of the [`parity-scale-codec`](https://crates.io/crates/parity-scale-codec) crate.
// TODO: Enable via feature?
pub mod scale {
    pub use parity_scale_codec::*;
}

pub type Balance = u128;

#[derive(Debug, Clone, Copy)]
pub enum Network {
    Polkadot,
    Kusama,
    Westend,
}

impl Network {
    pub fn genesis(&self) -> [u8; 32] {
        match self {
            Self::Polkadot => [0; 32],
            Self::Kusama => [0; 32],
            Self::Westend => [0; 32],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
// TODO: Custom Encode/Decode implementation. See https://substrate.dev/rustdocs/latest/sp_runtime/generic/enum.Era.html
pub enum Mortality {
    Immortal,
    Mortal((), ()),
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

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct AccountId32([u8; 32]);
