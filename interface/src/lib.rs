use blake2_rfc::blake2b::blake2b;
use parity_scale_codec::{Decode, Encode};
use schnorrkel::keys::Keypair as SrKeypair;
use schnorrkel::sign::Signature as SrSignature;
use schnorrkel::signing_context;

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
    #[gekko_generator::parse_from_hex_file("dumps/hex/metadata_polkadot_9050.hex")]
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

#[derive(Debug, Clone)]
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

        let extra = SignedExtra::new();
        let additional = AdditionalSigned::new();
        let payload = SignedPayload::from_parts(call, extra, additional);

        // TODO:
        let sig = match &signer {
            MultiSigner::Ed25519(_) => MultiSignature::Ed25519(Sig),
            MultiSigner::Sr25519(signer) => {
                let context = signing_context(b"substrate");
                let sig = payload.using_encoded(|payload| signer.sign(context.bytes(payload)));
                MultiSignature::Sr25519(sig.to_bytes())
            }
            MultiSigner::Ecdsa(_) => MultiSignature::Ecdsa(Sig),
        };

        let addr = signer.into();
        let (call, _, _) = payload.deconstruct();

        Ok(SignedExtrinsic {
            signature: Some((addr, sig, Extra)),
            function: call,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct SignedExtra {
    pub spec_version: (),
    pub tx_version: (),
    pub genesis: (),
    pub era: (),
    pub nonce: (),
    pub weight: (),
    pub payment: (),
    pub claims: (),
}

impl SignedExtra {
    pub fn new() -> Self {
        unimplemented!()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct AdditionalSigned {
    pub spec_version: (),
    pub tx_version: (),
    pub genesis: (),
    pub era: (),
    pub nonce: (),
    pub weight: (),
    pub payment: (),
    pub claims: (),
}

impl AdditionalSigned {
    pub fn new() -> Self {
        unimplemented!()
    }
}

pub struct SignedPayload<Call, Extra, AdditionalSigned> {
    pub call: Call,
    pub extra: Extra,
    pub additional: AdditionalSigned,
}

impl<Call, Extra, AdditionalSigned> SignedPayload<Call, Extra, AdditionalSigned> {
    fn from_parts(call: Call, extra: Extra, additional: AdditionalSigned) -> Self {
        SignedPayload {
            call: call,
            extra: extra,
            additional: additional,
        }
    }
    fn deconstruct(self) -> (Call, Extra, AdditionalSigned) {
        (self.call, self.extra, self.additional)
    }
}

impl<Call, Extra, AdditionalSigned> Encode for SignedPayload<Call, Extra, AdditionalSigned>
where
    Call: Encode,
    Extra: Encode,
    AdditionalSigned: Encode,
{
    fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
        (&self.call, &self.extra, &self.additional).using_encoded(|payload| {
            if payload.len() > 256 {
                f(blake2b(32, &[], &payload).as_bytes())
            } else {
                f(payload)
            }
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
    Sr25519([u8; 64]),
    Ecdsa(Sig),
}

impl From<MultiSigner> for MultiAddress<AccountId32, ()> {
    fn from(_signer: MultiSigner) -> Self {
        unimplemented!()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct Sig;

#[derive(Debug, Clone)]
pub enum MultiSigner {
    Ed25519(Public),
    Sr25519(SrKeypair),
    Ecdsa(Public),
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct Public;

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct AccountId32([u8; 32]);

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct Extra;