use parity_scale_codec::{Decode, Encode};

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
    #[gekko_generator::parse_from_hex_file("metadata/dumps/metadata_polkadot_9050.hex")]
    struct RM9050;
}

pub mod common {
    /// Re-export of the [`parity-scale-codec`](https://crates.io/crates/parity-scale-codec) crate.
    // TODO: Enable via feature?
    pub mod scale {
        pub use parity_scale_codec::*;
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub enum Error {
    BuilderError(String),
}

pub type PolkadotSignedExtrinsic<Call> =
    SignedExtrinsic<MultiAddress<AccountId32, ()>, Call, MultiSignature, Extra>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolkadotSignerBuilder<Call> {
    signer: Option<MultiSigner>,
    call: Option<Call>,
}

impl<Call: Encode> PolkadotSignerBuilder<Call> {
    pub fn new() -> Self {
        Self {
            signer: None,
            call: None,
        }
    }
    pub fn set_signer(self, signer: MultiSigner) -> Self {
        Self {
            signer: Some(signer),
            ..self
        }
    }
    pub fn set_call(self, call: Call) -> Self {
        Self {
            call: Some(call),
            ..self
        }
    }
    pub fn build(self) -> Result<PolkadotSignedExtrinsic<Call>> {
        let signer = self
            .signer
            .ok_or(Error::BuilderError("set_signer".to_string()))?;
        let call = self
            .call
            .ok_or(Error::BuilderError("set_call".to_string()))?;

        // TODO:
        let sig = match signer {
            MultiSigner::Ed25519(_) => MultiSignature::Ed25519(Sig),
            MultiSigner::Sr25519(_) => MultiSignature::Sr25519(Sig),
            MultiSigner::Ecdsa(_) => MultiSignature::Ecdsa(Sig),
        };

        let addr = signer.into();

        Ok(SignedExtrinsic {
            signature: Some((addr, sig, Extra)),
            function: call,
        })
    }
}

/// The signed extrinsic, aka. "UncheckedExtrinsic" in terms of substrate.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct SignedExtrinsic<Address, Call, Signature, Extra> {
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

impl From<MultiSigner> for MultiAddress<AccountId32, ()> {
    fn from(_signer: MultiSigner) -> Self {
        unimplemented!()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct Sig;

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub enum MultiSigner {
    Ed25519(Public),
    Sr25519(Public),
    Ecdsa(Public),
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct Public;

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct AccountId32([u8; 32]);

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct Extra;
