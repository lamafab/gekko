use crate::common::{AccountId, Balance, Mortality, MultiKeyPair, MultiSignature, Network};
use crate::runtime::{kusama, polkadot};
use crate::{blake2b, Error, Result};
use parity_scale_codec::{Decode, Encode, Error as ScaleError, Input};
use sp_core::crypto::Pair;

pub const TX_VERSION: u32 = 4;

/// A transaction that can contain a signature. Referred to as
/// "UncheckedExtrinsic" in Substrate vocabulary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction<Address, Call, Signature, ExtraSignaturePayload> {
    pub signature: Option<(Address, Signature, ExtraSignaturePayload)>,
    pub call: Call,
}

impl<Call> Transaction<(), Call, (), ()> {
    pub fn new_unsigned(call: Call) -> Self {
        Self {
            signature: None,
            call,
        }
    }
}

impl<Address, Call, Signature, ExtraSignaturePayload> Encode
    for Transaction<Address, Call, Signature, ExtraSignaturePayload>
where
    Address: Encode,
    Signature: Encode,
    Call: Encode,
    ExtraSignaturePayload: Encode,
{
    fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
        let mut enc: Vec<u8> = Vec::with_capacity(std::mem::size_of::<Self>());

        // Add version Id.
        match &self.signature {
            Some(sig) => {
                // First bit implies signed (1), remaining 7 bis
                // represent the TX_VERSION.
                enc.push(132);
                sig.encode_to(&mut enc);
            }
            None => {
                // First bit implies unsigned (0), remaining 7 bits
                // represent the TX_VERSION.
                enc.push(4);
            }
        }

        self.call.encode_to(&mut enc);
        f(&enc.encode())
    }
}

impl<Address, Call, Signature, ExtraSignaturePayload> Decode
    for Transaction<Address, Call, Signature, ExtraSignaturePayload>
where
    Address: Decode,
    Signature: Decode,
    Call: Decode,
    ExtraSignaturePayload: Decode,
{
    fn decode<I: Input>(input: &mut I) -> std::result::Result<Self, ScaleError> {
        // Throw away that compact integer which indicates the array length.
        let _: Vec<()> = Decode::decode(input)?;

        // Determine transaction version, handle signed/unsigned variant.
        // See the `Encode` implementation on why those values are used.
        let sig = match input.read_byte()? {
            132 => Some(Decode::decode(input)?),
            4 => None,
            _ => return Err("Invalid transaction version".into()),
        };

        Ok(Self {
            signature: sig,
            call: Decode::decode(input)?,
        })
    }
}

pub type PolkadotSignedExtrinsic<Call> = Transaction<AccountId, Call, MultiSignature, Payload>;

/// Builder type for creating signed transactions.
///
/// # Example
///
/// ```
/// use gekko::common::*;
/// use gekko::transaction::v4::*;
/// use gekko::runtime::polkadot::extrinsics::balances::TransferKeepAlive;
///
/// // In this example, a random key is generated. You probably want to *import* one.
/// let (keypair, _) = KeyPairBuilder::<Sr25519>::generate();
/// let currency = BalanceBuilder::new(Currency::Polkadot);
///
/// // The destination address.
/// let destination =
///     AccountId::from_ss58_address("12eDex4amEwj39T7Wz4Rkppb68YGCDYKG9QHhEhHGtNdDy7D")
///         .unwrap();
///
/// // Send 50 DOT to the destination.
/// let call = TransferKeepAlive {
///     dest: destination,
///     value: currency.balance(50),
/// };
///
/// // Transaction fee.
/// let payment = currency.balance_as_metric(Metric::Milli, 10).unwrap();
///
/// // Build the final transaction.
/// let transaction: PolkadotSignedExtrinsic<_> = SignedTransactionBuilder::new()
///     .signer(keypair)
///     .call(call)
///     .nonce(0)
///     .payment(payment)
///     .network(Network::Polkadot)
///     .spec_version(9080)
///     .build()
///     .unwrap();
/// ```
#[derive(Clone)]
pub struct SignedTransactionBuilder<Call> {
    signer: Option<MultiKeyPair>,
    call: Option<Call>,
    nonce: Option<u32>,
    payment: Option<u128>,
    network: Option<Network>,
    mortality: Mortality,
    spec_version: Option<u32>,
}

impl<Call> Default for SignedTransactionBuilder<Call> {
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

impl<Call: Encode> SignedTransactionBuilder<Call> {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn signer<T: Into<MultiKeyPair>>(self, signer: T) -> Self {
        Self {
            signer: Some(signer.into()),
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
    // TODO: Rename to "fee"
    pub fn payment(self, payment: Balance) -> Self {
        Self {
            payment: Some(payment.as_base_unit()),
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

        // Determine spec_version.
        let spec_version = match network {
            Network::Kusama => self.spec_version.unwrap_or(kusama::SPEC_VERSION),
            Network::Polkadot => self.spec_version.unwrap_or(polkadot::SPEC_VERSION),
            // `spec_version` must be provided for any other network.
            _ => self
                .spec_version
                .ok_or(Error::BuilderMissingField("spec_version"))?,
        };

        // Set mortality starting period.
        let birth = match self.mortality {
            Mortality::Immortal => network.genesis(),
            Mortality::Mortal(_, _, birth) => {
                birth.ok_or(Error::BuilderMissingField("no birth block in Mortality"))?
            }
        };

        // Prepare transaction payload.
        let payload = Payload {
            mortality: self.mortality,
            nonce: nonce,
            payment: payment,
        };

        let extra = ExtraSignaturePayload {
            spec_version: spec_version,
            tx_version: TX_VERSION,
            genesis: network.genesis(),
            birth: birth,
        };

        // Create the full signature payload.
        let sig_payload = SignaturePayload::new(call, payload, extra);

        // Create signature.
        let sig = sig_payload.using_encoded(|payload| match &signer {
            MultiKeyPair::Ed25519(pair) => pair.sign(payload).into(),
            MultiKeyPair::Sr25519(pair) => pair.sign(payload).into(),
            MultiKeyPair::Ecdsa(pair) => pair.sign(payload).into(),
        });

        // Prepare all entries for the final extrinsic.
        let addr = signer.into();
        let (call, payload, _) = sig_payload.deconstruct();

        Ok(Transaction {
            signature: Some((addr, sig, payload)),
            call: call,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct Payload {
    pub mortality: Mortality,
    #[codec(compact)]
    pub nonce: u32,
    #[codec(compact)]
    pub payment: u128,
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct ExtraSignaturePayload {
    pub spec_version: u32,
    pub tx_version: u32,
    pub genesis: [u8; 32],
    /// The block hash from where the period of mortality begins. If the
    /// transaction is immortal, it's the genesis hash. See [Mortality] for more
    /// information.
    pub birth: [u8; 32],
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
                f(&blake2b(&payload))
            } else {
                f(payload)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::*;
    use std::env;

    #[derive(Debug, Eq, PartialEq, Encode, Decode)]
    struct SomeExtrinsic {
        a: u32,
        b: String,
        c: Vec<u32>,
    }

    #[test]
    fn unsigned_transaction_encode_decode() {
        let call = SomeExtrinsic {
            a: 10,
            b: "some".to_string(),
            c: vec![20, 30, 40],
        };

        let transaction = Transaction::new_unsigned(call);

        let encoded = transaction.encode();
        let decoded = Decode::decode(&mut encoded.as_ref()).unwrap();

        assert_eq!(transaction, decoded);
    }

    #[test]
    fn signed_transaction_encode_decode() {
        let (keypair, _) = KeyPairBuilder::<Sr25519>::generate();

        let call = SomeExtrinsic {
            a: 10,
            b: "some".to_string(),
            c: vec![20, 30, 40],
        };

        // Transaction fee.
        let payment = BalanceBuilder::new(Currency::Westend)
            .balance_as_metric(Metric::Milli, 500)
            .unwrap();

        let transaction: PolkadotSignedExtrinsic<_> = SignedTransactionBuilder::new()
            .signer(keypair)
            .call(call)
            .nonce(0)
            .payment(payment)
            .network(Network::Polkadot)
            .build()
            .unwrap();

        let encoded = transaction.encode();
        let decoded = Decode::decode(&mut encoded.as_ref()).unwrap();

        assert_eq!(transaction, decoded);
    }

    #[test]
    #[ignore]
    fn westend_create_signed_extrinsic() {
        use crate::runtime::kusama::extrinsics::balances::TransferKeepAlive;

        let mut seed = [0; 32];
        seed.copy_from_slice(
            &mut hex::decode(env::var("WESTEND_SEED").unwrap().as_bytes()).unwrap(),
        );

        let keypair = KeyPairBuilder::<Sr25519>::from_seed(&seed);
        let currency = BalanceBuilder::new(Currency::Westend);
        let destination =
            AccountId::from_ss58_address("5G3j1t2Ho1e4MfiLvce9xEXWjmJSpExoxAbPp5aGDjerS9nC")
                .unwrap();

        let call = TransferKeepAlive {
            dest: destination,
            value: currency.balance(1),
        };

        println!("CALL >> 0x{}", hex::encode(&call.encode()));

        // Transaction fee.
        let payment = currency.balance_as_metric(Metric::Milli, 500).unwrap();

        let transaction: PolkadotSignedExtrinsic<_> = SignedTransactionBuilder::new()
            .signer(keypair)
            .call(call)
            .nonce(0)
            .payment(payment)
            .network(Network::Westend)
            .spec_version(9080)
            .build()
            .unwrap();

        println!(
            "SIGNED TRANSACTION >> 0x{}",
            hex::encode(&transaction.encode())
        );
    }
}
