use crate::common::{
    AccountId32, Balance, Mortality, MultiAddress, MultiSignature, MultiSigner, Network,
};
use crate::{Error, Result};
use blake2_rfc::blake2b::blake2b;
use ed25519_dalek::Signer;
use parity_scale_codec::{Decode, Encode};
use schnorrkel::signing_context;
use secp256k1::Message;

// TODO:
const SPEC_VERSION: u32 = 0;
const TX_VERSION: u32 = 4;

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
pub struct ExtrinsicBuilder<Call> {
    signer: Option<MultiSigner>,
    call: Option<Call>,
    nonce: Option<u32>,
    // TODO: Create "Balance" alias
    payment: Option<Balance>,
    network: Option<Network>,
    raw_genesis: Option<[u8; 32]>,
    mortality: Mortality,
    spec_version: u32,
}

impl<Call> Default for ExtrinsicBuilder<Call> {
    fn default() -> Self {
        Self {
            signer: None,
            call: None,
            nonce: None,
            payment: None,
            network: None,
            raw_genesis: None,
            mortality: Mortality::Immortal,
            spec_version: SPEC_VERSION,
        }
    }
}

impl<Call: Encode> ExtrinsicBuilder<Call> {
    pub fn new() -> Self {
        Default::default()
    }
    // TODO: should be Into<MultiAddress>
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
    pub fn nonce(self, nonce: u32) -> Self {
        Self {
            nonce: Some(nonce),
            ..self
        }
    }
    pub fn payment(self, payment: Balance) -> Self {
        Self {
            payment: Some(payment),
            ..self
        }
    }
    pub fn network(self, network: Network) -> Self {
        Self {
            network: Some(network),
            ..self
        }
    }
    pub fn raw_genesis(self, genesis: [u8; 32]) -> Self {
        Self {
            raw_genesis: Some(genesis),
            ..self
        }
    }
    pub fn mortality(self, mortality: Mortality) -> Self {
        Self {
            mortality: mortality,
            ..self
        }
    }
    pub fn spec_version(self, version: u32) -> Self {
        Self {
            spec_version: version,
            ..self
        }
    }
    pub fn build(self) -> Result<PolkadotSignedExtrinsic<Call>> {
        let signer = self.signer.ok_or(Error::BuilderMissingField("signer"))?;
        let call = self.call.ok_or(Error::BuilderMissingField("call"))?;
        let nonce = self.nonce.ok_or(Error::BuilderMissingField("nonce"))?;
        let payment = self.payment.ok_or(Error::BuilderMissingField("payment"))?;

        // Prepare transaction payload.
        let payload = Payload {
            mortality: self.mortality,
            nonce: nonce,
            payment: payment,
        };

        // Prepare extra signature payload.
        let genesis = {
            match (self.network, self.raw_genesis) {
                (Some(_), Some(_)) => {
                    return Err(Error::BuilderContradictingEntries("network", "raw_genesis"));
                }
                (Some(network), None) => network.genesis(),
                (None, Some(raw_genesis)) => raw_genesis,
                (None, None) => return Err(Error::BuilderMissingField("network")),
            }
        };

        let mortality = match self.mortality {
            Mortality::Immortal => genesis,
            Mortality::Mortal(_, _) => unimplemented!(),
        };

        let extra = ExtraSignaturePayload {
            spec_version: self.spec_version,
            tx_version: TX_VERSION,
            genesis: genesis,
            mortality: mortality,
        };

        // Create the full signature payload.
        let sig_payload = SignaturePayload::new(call, payload, extra);

        // Create signature.
        let sig = match &signer {
            MultiSigner::Ed25519(signer) => {
                let sig = sig_payload.using_encoded(|payload| signer.sign(payload));
                MultiSignature::Ed25519(sig.to_bytes())
            }
            MultiSigner::Sr25519(signer) => {
                let context = signing_context(b"substrate");
                let sig = sig_payload.using_encoded(|payload| signer.sign(context.bytes(payload)));
                MultiSignature::Sr25519(sig.to_bytes())
            }
            MultiSigner::Ecdsa(signer) => {
                let sig = sig_payload.using_encoded(|payload| {
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

        // Prepare all entries for the final extrinsic.
        let addr = signer.into();
        let (call, payload, _) = sig_payload.deconstruct();

        Ok(SignedExtrinsic {
            signature: Some((addr, sig, payload)),
            function: call,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct Payload {
    pub mortality: Mortality,
    #[codec(compact)]
    pub nonce: u32,
    #[codec(compact)]
    pub payment: Balance,
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct ExtraSignaturePayload {
    pub spec_version: u32,
    pub tx_version: u32,
    pub genesis: [u8; 32],
    pub mortality: [u8; 32],
}

pub struct SignaturePayload<Call, Payload, ExtraSignaturePayload> {
    pub call: Call,
    pub payload: Payload,
    pub extra: ExtraSignaturePayload,
}

impl<Call, Payload, ExtraSignaturePayload> SignaturePayload<Call, Payload, ExtraSignaturePayload> {
    fn new(call: Call, payload: Payload, extra: ExtraSignaturePayload) -> Self {
        SignaturePayload {
            call: call,
            payload: payload,
            extra: extra,
        }
    }
    fn deconstruct(self) -> (Call, Payload, ExtraSignaturePayload) {
        (self.call, self.payload, self.extra)
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
        (&self.call, &self.payload, &self.extra).using_encoded(|payload| {
            if payload.len() > 256 {
                f(blake2b(32, &[], &payload).as_bytes())
            } else {
                f(payload)
            }
        })
    }
}
