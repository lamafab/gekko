use crate::common::{AccountId32, Era, MultiAddress, MultiSignature, MultiSigner};
use crate::{Error, Result};
use blake2_rfc::blake2b::blake2b;
use ed25519_dalek::{Keypair as EdKeypair, Signer};
use parity_scale_codec::{Decode, Encode};
use schnorrkel::keys::Keypair as SrKeypair;
use schnorrkel::signing_context;
use secp256k1::{Message, SecretKey};

/// The signed extrinsic, aka. "UncheckedExtrinsic" in terms of substrate.
// TODO: This requires a custom Encode/Decode implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedExtrinsic<Address, Call, Signature, ExtraSignaturePayload> {
    pub signature: Option<(Address, Signature, ExtraSignaturePayload)>,
    pub function: Call,
}

pub type PolkadotSignedExtrinsic<Call> =
    SignedExtrinsic<MultiAddress<AccountId32, ()>, Call, MultiSignature, Payload>;

#[derive(Debug)]
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
    pub fn signer(self, signer: MultiSigner) -> Self {
        Self {
            signer: Some(signer),
            ..self
        }
    }
    pub fn call(self, call: Call) -> Self {
        Self {
            call: Some(call),
            ..self
        }
    }
    pub fn build(self) -> Result<PolkadotSignedExtrinsic<Call>> {
        let signer = self
            .signer
            .ok_or(Error::BuilderError("signer".to_string()))?;
        let call = self.call.ok_or(Error::BuilderError("call".to_string()))?;

        let payload = PayloadBuilder::new().build()?;
        let additional = ExtraSignaturePayload::new();
        let payload = SignaturePayload::from_parts(call, payload, additional);

        // TODO:
        let sig = match &signer {
            MultiSigner::Ed25519(signer) => {
                let sig = payload.using_encoded(|payload| signer.sign(payload));
                MultiSignature::Ed25519(sig.to_bytes())
            }
            MultiSigner::Sr25519(signer) => {
                let context = signing_context(b"substrate");
                let sig = payload.using_encoded(|payload| signer.sign(context.bytes(payload)));
                MultiSignature::Sr25519(sig.to_bytes())
            }
            MultiSigner::Ecdsa(signer) => {
                let sig = payload.using_encoded(|payload| {
                    let mut message: [u8; 32] = [0; 32];
                    message.copy_from_slice(&blake2b(32, &[], &payload).as_bytes());

                    let parsed = Message::parse(&message);
                    let (sig, rec) = secp256k1::sign(&parsed, &signer);

                    let mut serialized: [u8; 65] = [0; 65];
                    serialized[..65].copy_from_slice(&sig.serialize());
                    serialized[65] = rec.serialize();
                    serialized
                });

                MultiSignature::Ecdsa(sig)
            }
        };

        let addr = signer.into();
        let (call, payload, _) = payload.deconstruct();

        Ok(SignedExtrinsic {
            signature: Some((addr, sig, payload)),
            function: call,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct Payload {
    pub mortality: Era,
    #[codec(compact)]
    pub nonce: u32,
    #[codec(compact)]
    pub payment: u128,
}

pub struct PayloadBuilder {
    mortality: Era,
    nonce: Option<u32>,
    payment: Option<u128>,
}

impl PayloadBuilder {
    pub fn new() -> Self {
        Self {
            mortality: Era::Immortal,
            nonce: None,
            payment: None,
        }
    }
    pub fn mortality(self, era: Era) -> Self {
        Self {
            mortality: era,
            ..self
        }
    }
    pub fn nonce(self, nonce: u32) -> Self {
        Self {
            nonce: Some(nonce),
            ..self
        }
    }
    // TODO: Add a better way to specify balances.
    pub fn payment(self, balance: u128) -> Self {
        Self {
            payment: Some(balance),
            ..self
        }
    }
    #[rustfmt::skip]
    pub fn build(self) -> Result<Payload> {
        Ok(Payload {
            mortality: self.mortality,
            nonce: self
                .nonce
                .ok_or(Error::BuilderError("nonce".to_string()))?,
            payment: self
                .payment
                .ok_or(Error::BuilderError("payment".to_string()))?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct ExtraSignaturePayload {
    pub spec_version: u32,
    pub tx_version: u32,
    pub genesis: [u8; 32],
    pub mortality: [u8; 32],
}

impl ExtraSignaturePayload {
    pub fn new() -> Self {
        unimplemented!()
    }
}

pub struct AdditionalPayloadBuilder {
    spec_version: Option<u32>,
    tx_version: Option<u32>,
    genesis: Option<[u8; 32]>,
    mortality: Option<[u8; 32]>,
}

impl AdditionalPayloadBuilder {
    pub fn new() -> Self {
        Self {
            spec_version: None,
            tx_version: None,
            genesis: None,
            era: None,
        }
    }
    pub fn spec_version(self, version: u32) -> Self {
	    Self { spec_version: Some(version), ..self }
    }
    pub fn tx_version(self, version: u32) -> Self {
	    Self { tx_version: Some(version), ..self }
    }
    pub fn genesis<T: Into<[u8; 32]>>(self, genesis: T) -> Self {
	    Self { genesis: Some(genesis.into()), ..self }
    }
    #[rustfmt::skip]
    pub fn build(self) -> Result<ExtraSignaturePayload> {
        Ok(ExtraSignaturePayload {
            spec_version: self
                .spec_version
                .ok_or(Error::BuilderError("spec_version".to_string()))?,
            tx_version: self
                .tx_version
                .ok_or(Error::BuilderError("tx_version".to_string()))?,
            genesis: self
                .genesis
                .ok_or(Error::BuilderError("genesis".to_string()))?,
            era: self
                .era
                .ok_or(Error::BuilderError("era".to_string()))?,
        })
    }
}

pub struct SignaturePayload<Call, Payload, ExtraSignaturePayload> {
    pub call: Call,
    pub payload: Payload,
    pub additional: ExtraSignaturePayload,
}

impl<Call, Payload, ExtraSignaturePayload> SignaturePayload<Call, Payload, ExtraSignaturePayload> {
    fn from_parts(call: Call, payload: Payload, additional: ExtraSignaturePayload) -> Self {
        SignaturePayload {
            call: call,
            payload: payload,
            additional: additional,
        }
    }
    fn deconstruct(self) -> (Call, Payload, ExtraSignaturePayload) {
        (self.call, self.payload, self.additional)
    }
}

impl<Call, Payload, ExtraSignaturePayload> Encode
    for SignaturePayload<Call, Payload, ExtraSignaturePayload>
where
    Call: Encode,
    Payload: Encode,
    ExtraSignaturePayload: Encode,
{
    fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
        (&self.call, &self.payload, &self.additional).using_encoded(|payload| {
            if payload.len() > 256 {
                f(blake2b(32, &[], &payload).as_bytes())
            } else {
                f(payload)
            }
        })
    }
}
