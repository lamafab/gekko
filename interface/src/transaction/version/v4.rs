use crate::common::{
    AccountId32, Balance, Mortality, MultiAddress, MultiSignature, MultiSigner, Network,
};
use crate::runtime::{kusama, polkadot};
use crate::{Error, Result};
use blake2_rfc::blake2b::blake2b;
use ed25519_dalek::Signer;
use parity_scale_codec::{Decode, Encode};
use schnorrkel::signing_context;
use secp256k1::Message;

pub const TX_VERSION: u32 = 4;

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
    payment: Option<Balance>,
    network: Option<Network>,
    mortality: Mortality,
    spec_version: Option<u32>,
}

impl<Call> Default for ExtrinsicBuilder<Call> {
    fn default() -> Self {
        Self {
            signer: None,
            call: None,
            nonce: None,
            payment: None,
            network: None,
            mortality: Mortality::Immortal,
            spec_version: None,
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
    /// Set the extrinsic this transaction must call. Available extrinsic
    /// interfaces are located in the [runtime](crate::runtime) module. This
    /// function accepts any type which implements [the SCALE codec](Encode).
    pub fn call(self, call: Call) -> Self {
        Self {
            call: Some(call),
            ..self
        }
    }
    /// Set the nonce of the transaction. You must track and increment the nonce
    /// of the corresponding signer manually, retrieved from the blockchain.
    /// Keep pending transactions in mind.
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
    /// Set the network this transaction is for.
    pub fn network(self, network: Network) -> Self {
        Self {
            network: Some(network),
            ..self
        }
    }
    /// Set the mortality of the transaction. Immortal by default.
    pub fn mortality(self, mortality: Mortality) -> Self {
        Self {
            mortality: mortality,
            ..self
        }
    }
    /// Set the `spec_version` of the runtime. For Kusama and Polkadot,
    /// the builder uses the **latest** known versions by default:
    /// [kusama::LATEST_SPEC_VERSION] and [polkadot::LATEST_SPEC_VERSION],
    /// respectively.
    ///
    /// For any other [Network], calling this function is required.
    pub fn spec_version(self, version: u32) -> Self {
        Self {
            spec_version: Some(version),
            ..self
        }
    }
    pub fn build(self) -> Result<PolkadotSignedExtrinsic<Call>> {
        let signer = self.signer.ok_or(Error::BuilderMissingField("signer"))?;
        let call = self.call.ok_or(Error::BuilderMissingField("call"))?;
        let nonce = self.nonce.ok_or(Error::BuilderMissingField("nonce"))?;
        let payment = self.payment.ok_or(Error::BuilderMissingField("payment"))?;
        let network = self.network.ok_or(Error::BuilderMissingField("network"))?;

        // Prepare transaction payload.
        let payload = Payload {
            mortality: self.mortality,
            nonce: nonce,
            payment: payment,
        };

        // Determine spec_version.
        let spec_version = match network {
            Network::Kusama => self.spec_version.unwrap_or(kusama::LATEST_SPEC_VERSION),
            Network::Polkadot => self.spec_version.unwrap_or(polkadot::LATEST_SPEC_VERSION),
            // `spec_version` must be provided for any other network
            _ => self
                .spec_version
                .ok_or(Error::BuilderMissingField("spec_version"))?,
        };

        let mortality = match self.mortality {
            Mortality::Immortal => network.genesis(),
            Mortality::Mortal(birth) => birth,
        };

        let extra = ExtraSignaturePayload {
            spec_version: spec_version,
            tx_version: TX_VERSION,
            genesis: network.genesis(),
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
    /// The block hash from where the period of mortality begins. If the
    /// transaction is immortal, it's the genesis hash. See [Mortality] for more
    /// information.
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
            if payload.len() > 32 {
                f(blake2b(32, &[], &payload).as_bytes())
            } else {
                f(payload)
            }
        })
    }
}
