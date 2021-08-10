use self::ss58format::{Ss58AddressFormat, Ss58Codec};
use crate::{blake2b, Error};
use ed25519_dalek::Signer;
use parity_scale_codec::{Decode, Encode};
use rand::rngs::OsRng;
use schnorrkel::keys::{MINI_SECRET_KEY_LENGTH, SECRET_KEY_LENGTH};
use schnorrkel::{signing_context, ExpansionMode, MiniSecretKey};
use secp256k1::{Message, Secp256k1};

pub mod ss58format;
/// Re-export of the [`parity-scale-codec`](https://crates.io/crates/parity-scale-codec) crate.
pub mod scale {
    pub use parity_scale_codec::*;
}
pub mod crypto {
    pub use ed25519_dalek as ed25519;
    pub use schnorrkel as sr25519;
    pub use secp256k1;
}

type Result<T> = std::result::Result<T, PrimitiveError>;

pub type Balance = u128;

#[derive(Debug)]
pub enum PrimitiveError {
    // Related to signature or keypair.
    SchnorrkelSignature(schnorrkel::SignatureError),
    Ed25519Signature(ed25519_dalek::SignatureError),
    Secp256k1Signature(secp256k1::Error),
    InvalidSignature,
    InvalidKeySignatureMatch,
}


impl From<schnorrkel::SignatureError> for PrimitiveError {
    fn from(val: schnorrkel::SignatureError) -> Self {
        PrimitiveError::SchnorrkelSignature(val)
    }
}

impl From<ed25519_dalek::SignatureError> for PrimitiveError {
    fn from(val: ed25519_dalek::SignatureError) -> Self {
        PrimitiveError::Ed25519Signature(val)
    }
}

impl From<secp256k1::Error> for PrimitiveError {
    fn from(val: secp256k1::Error) -> Self {
        PrimitiveError::Secp256k1Signature(val)
    }
}

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

// TODO: Deprecate, just use AccountId
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

impl From<MultiKeyPair> for MultiAddress<AccountId32, ()> {
    fn from(_signer: MultiKeyPair) -> Self {
        unimplemented!()
    }
}

#[derive(Debug)]
pub struct Sr25519KeyPair(schnorrkel::keys::Keypair);

impl Sr25519KeyPair {
    const SIGNING_CONTEXT: &'static str = "substrate";

    pub fn new() -> Self {
        Sr25519KeyPair(schnorrkel::keys::Keypair::generate())
    }
    pub fn from_seed(seed: &[u8]) -> Result<Self> {
        let pair = match seed.len() {
            MINI_SECRET_KEY_LENGTH => {
                MiniSecretKey::from_bytes(seed)?.expand_to_keypair(ExpansionMode::Ed25519)
            }
            SECRET_KEY_LENGTH => schnorrkel::SecretKey::from_bytes(seed)?.to_keypair(),
            _ => panic!(),
        };

        Ok(Sr25519KeyPair(pair))
    }
    pub fn sign_simple<T: AsRef<[u8]>>(&self, message: T) -> [u8; 64] {
        let context = signing_context(Self::SIGNING_CONTEXT.as_bytes());
        self.0.sign(context.bytes(message.as_ref())).to_bytes()
    }
    pub fn verify_simple<T: AsRef<[u8]>, S: AsRef<[u8]>>(
        &self,
        message: T,
        signature: S,
    ) -> Result<()> {
        let sig_parsed = schnorrkel::sign::Signature::from_bytes(signature.as_ref())?;
        let context = signing_context(Self::SIGNING_CONTEXT.as_bytes());

        self.0
            .public
            .verify(context.bytes(message.as_ref()), &sig_parsed)
            .map_err(|_| PrimitiveError::InvalidSignature)
    }
    /// Consumes the keypair into the underlying type. The Sr25519 library is
    /// exposed in the [common::crypto](crypto) module.
    pub fn into_inner(self) -> schnorrkel::keys::Keypair {
        self.0
    }
}

#[derive(Debug)]
pub struct Ed25519KeyPair(ed25519_dalek::Keypair);

impl Ed25519KeyPair {
    pub fn new() -> Self {
        Ed25519KeyPair(ed25519_dalek::Keypair::generate(&mut OsRng))
    }
    pub fn from_seed(seed: &[u8]) -> Result<Self> {
        let secret = ed25519_dalek::SecretKey::from_bytes(seed)?;
        let public = ed25519_dalek::PublicKey::from(&secret);
        Ok(Ed25519KeyPair(ed25519_dalek::Keypair { secret, public }))
    }
    pub fn sign_simple<T: AsRef<[u8]>>(&self, message: T) -> [u8; 64] {
        self.0.sign(message.as_ref()).to_bytes()
    }
    pub fn verify_simple<T: AsRef<[u8]>, S: AsRef<[u8]>>(
        &self,
        message: T,
        signature: S,
    ) -> Result<()> {
        let sig = signature.as_ref();
        if sig.len() != 64 {
            // TODO
            panic!()
        }

        let mut buffer = [0; 64];
        buffer.copy_from_slice(sig);

        ed25519_dalek::Verifier::verify(
            &self.0,
            message.as_ref(),
            &ed25519_dalek::Signature::new(buffer),
        )
        .map_err(|_| PrimitiveError::InvalidSignature)
    }
    /// Consumes the keypair into the underlying type. The Ed25519 library is
    /// exposed in the [common::crypto](crypto) module.
    pub fn into_inner(self) -> ed25519_dalek::Keypair {
        self.0
    }
}

#[derive(Debug)]
pub struct EcdsaKeyPair {
    secret: secp256k1::SecretKey,
    public: secp256k1::PublicKey,
}

impl EcdsaKeyPair {
    pub fn new() -> Self {
        let engine = secp256k1::Secp256k1::signing_only();
        let mut os_rng =
            secp256k1::rand::rngs::OsRng::new().expect("Failed to generate random seed from OS");

        let (secret, public) = engine.generate_keypair(&mut os_rng);

        EcdsaKeyPair {
            secret: secret,
            public: public,
        }
    }
    pub fn from_seed(seed: &[u8]) -> Result<Self> {
        let secret = secp256k1::SecretKey::from_slice(seed)?;

        let engine = secp256k1::Secp256k1::signing_only();
        let public = secp256k1::PublicKey::from_secret_key(&engine, &secret);
        Ok(EcdsaKeyPair {
            secret: secret,
            public: public,
        })
    }
    pub fn sign_simple<T: AsRef<[u8]>>(&self, message: T) -> [u8; 65] {
        // Message must be 32-bytes.
        let message = blake2b(&message.as_ref());

        let parsed = Message::from_slice(&message).unwrap();
        let (recovery, sig) = Secp256k1::signing_only()
            .sign_recoverable(&parsed, &self.secret)
            .serialize_compact();

        let mut serialized: [u8; 65] = [0; 65];
        serialized[..64].copy_from_slice(&sig);
        serialized[64] = recovery.to_i32() as u8;
        serialized
    }
    pub fn verify_simple<T: AsRef<[u8]>, S: AsRef<[u8]>>(
        &self,
        message: T,
        signature: S,
    ) -> Result<()> {
        // Message must be 32-bytes.
        let message = secp256k1::Message::from_slice(&blake2b(message.as_ref())).unwrap();

        // TODO: Error message should specify compact encoding.
        let sig = signature.as_ref();
        let sig = secp256k1::Signature::from_compact({
            // Skip recovery byte, if present.
            if sig.len() == 65 {
                &sig[0..64]
            } else {
                &sig
            }
        })?;

        let engine = secp256k1::Secp256k1::verification_only();
        engine.verify(&message, &sig, &self.public).map_err(|_| PrimitiveError::InvalidSignature)
    }
    /// Consumes the keypair into the underlying type. The ECDSA library is
    /// exposed in the [common::crypto](crypto) module.
    pub fn into_inner(self) -> (secp256k1::SecretKey, secp256k1::PublicKey) {
        (self.secret, self.public)
    }
}

#[derive(Debug)]
pub enum MultiKeyPair {
    Ed25519(Ed25519KeyPair),
    Sr25519(Sr25519KeyPair),
    Ecdsa(EcdsaKeyPair),
}

impl MultiKeyPair {
    pub fn to_public_key(&self) -> Vec<u8> {
        // This method returns a vector rather than an array since public key
        // sizes vary in size.
        match self {
            Self::Ed25519(pair) => pair.0.public.to_bytes().to_vec(),
            Self::Sr25519(pair) => pair.0.public.to_bytes().to_vec(),
            Self::Ecdsa(pair) => pair.public.serialize().to_vec(),
        }
    }
    pub fn to_account_id(&self) -> AccountId32 {
        let pub_key = match self {
            Self::Ed25519(pair) => pair.0.public.to_bytes(),
            Self::Sr25519(pair) => pair.0.public.to_bytes(),
            Self::Ecdsa(pair) => {
                // Hashed, since the ECDSA public key is 33 bytes.
                blake2b(pair.public.serialize())
            }
        };

        AccountId32(pub_key)
    }
    pub fn to_ss58_address(&self, format: Ss58AddressFormat) -> String {
        self.to_account_id().to_ss58_address(format)
    }
    pub fn sign_simple<T: AsRef<[u8]>>(&self, message: T) -> MultiSignature {
        match self {
            Self::Ed25519(signer) => MultiSignature::Ed25519(signer.sign_simple(message)),
            Self::Sr25519(signer) => MultiSignature::Sr25519(signer.sign_simple(message)),
            Self::Ecdsa(signer) => MultiSignature::Ecdsa(signer.sign_simple(message)),
        }
    }
    pub fn verify_simple<T: AsRef<[u8]>>(
        &self,
        message: T,
        signature: MultiSignature,
    ) -> Result<()> {
        match (self, signature) {
            (Self::Ed25519(signer), MultiSignature::Ed25519(sig)) => {
                signer.verify_simple(message.as_ref(), sig)
            }
            (Self::Sr25519(signer), MultiSignature::Sr25519(sig)) => {
                signer.verify_simple(message.as_ref(), sig)
            }
            (Self::Ecdsa(signer), MultiSignature::Ecdsa(sig)) => {
                signer.verify_simple(message.as_ref(), sig)
            }
            _ => Err(PrimitiveError::InvalidKeySignatureMatch),
        }
    }
}

impl From<Sr25519KeyPair> for MultiKeyPair {
    fn from(val: Sr25519KeyPair) -> Self {
        MultiKeyPair::Sr25519(val)
    }
}

impl From<Ed25519KeyPair> for MultiKeyPair {
    fn from(val: Ed25519KeyPair) -> Self {
        MultiKeyPair::Ed25519(val)
    }
}

impl From<EcdsaKeyPair> for MultiKeyPair {
    fn from(val: EcdsaKeyPair) -> Self {
        MultiKeyPair::Ecdsa(val)
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sr25519_from_seed() {}
}
