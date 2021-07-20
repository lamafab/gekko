#[macro_use]
extern crate parity_scale_codec;

pub use latest::*;

#[cfg(feature = "generator")]
pub mod generator {
    pub use gekko_generator::*;
}

#[cfg(feature = "metadata")]
pub mod metadata {
    pub use gekko_metadata::*;
}

pub mod latest {
    #[gekko_generator::parse_from_file("metadata/dumps/metadata_polkadot_9050.json")]
    struct RM9050;
}

pub mod common {
    /// Re-export of the [`parity-scale-codec`](https://crates.io/crates/parity-scale-codec) crate.
    // TODO: Enable via feature?
    pub mod scale {
        pub use parity_scale_codec::*;
    }
}

pub type PolkadotSignedExtrinsic =
    SignedExtrinsic<MultiAddress<AccountId32, ()>, MultiSignature, Extra>;

/// The signed extrinsic, aka. "UncheckedExtrinsic" in terms of substrate.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct SignedExtrinsic<Address, Signature, Extra> {
    pub signature: Option<(Address, Signature, Extra)>,
    pub function: Call,
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
    Ed25519(Sig),
    Sr25519(Sig),
    Ecdsa(Sig),
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct Sig {}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub enum MultiSigner {
    Ed25519(Public),
    Sr25519(Public),
    Ecdsa(Public),
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct Public {}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct AccountId32([u8; 32]);

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct Extra {}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct Call {}
