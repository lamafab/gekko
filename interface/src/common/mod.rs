//! This module contains useful primitives when working with the
//! [runtime](gekko::runtime).

use parity_scale_codec::{Compact, Decode, Encode, Input};
use sp_core::crypto::{AccountId32, Pair, Ss58AddressFormat, Ss58Codec};

pub extern crate parity_scale_codec as scale;
pub extern crate sp_core;

pub type Sr25519 = sp_core::sr25519::Pair;
pub type Ed25519 = sp_core::ed25519::Pair;
pub type Ecdsa = sp_core::ecdsa::Pair;

#[derive(Debug, Clone, Copy)]
// TODO: Rename to "Chain" or "Blockchain"?
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
            Self::Polkadot => "91b171bb158e2d3848fa23a9f1c25182fb8e20313b2c1eb49219da7a70ce90c3",
            Self::Kusama => "b0a8d493285c2df73290dfb7e61f870f17b41801197a149ca93654499ea3dafe",
            Self::Westend => "e143f23803ac50e8f6f8e62695d1ce9e4e1d68aa36c1cd2cfd15340213f3423e",
            Self::Custom(genesis) => return *genesis,
        };

        hex::decode_to_slice(hash_str, &mut genesis).unwrap();
        genesis
    }
}

pub enum Currency {
    Kusama,
    Polkadot,
    Westend,
    Custom(u128),
}

impl Currency {
    pub fn base_unit(&self) -> u128 {
        match self {
            Self::Kusama | Self::Westend => 1_000_000_000_000,
            Self::Polkadot => 10_000_000_000,
            Self::Custom(unit) => *unit,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct BalanceBuilder;

impl BalanceBuilder {
    pub fn new(currency: Currency) -> BalanceWithUnit {
        // TODO: Make sure `unit` is never zero.

        BalanceWithUnit {
            unit: currency.base_unit(),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct BalanceWithUnit {
    unit: u128,
}

impl BalanceWithUnit {
    // TODO: Consider removing this. Metric should be explicit.
    pub fn balance(self, balance: u128) -> Balance {
        self.balance_as_metric(Metric::One, balance).unwrap()
    }
    // TODO: Rename. TODO: Should return Result
    pub fn balance_as_metric(self, metric: Metric, balance: u128) -> Option<Balance> {
        Some(Balance {
            balance: convert_metrics(metric, Metric::One, balance.saturating_mul(self.unit))?,
            unit: self.unit,
        })
    }
}

pub struct OpaqueBalance;

/// Represents the balance of a chains native currency with metric conversion
/// utilities.
///
/// When creating or processing transactions, this types should be used to
/// reliably handle balances and to do metric conversions. The
/// [`Encode`](gekko::common::scale::Encode) implementation handles encoding to
/// [`Compact`](gekko::common::scale::Compact) automatically. Decoding is not
/// supported for this type since the base unit cannot be know without having
/// more context. For decoding, [`OpaqueBalance`] should be used instead.
///
/// # Example
///
/// ```
/// use gekko::common::*;
/// use gekko::runtime::polkadot::extrinsics::balances::TransferKeepAlive;
///
/// let destination =
///     AccountId::from_ss58_address("12eDex4amEwj39T7Wz4Rkppb68YGCDYKG9QHhEhHGtNdDy7D")
///         .unwrap();
///
/// let balance = BalanceBuilder::new(Currency::Polkadot).balance(50);
///
/// // Create a `transfer_keep_alive` extrinsic.
/// let call = TransferKeepAlive {
///     dest: destination,
///     value: balance,
/// };
/// ```
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Balance {
    balance: u128,
    unit: u128,
}

impl Balance {
    /// Converts the balance into the base unit the **runtime** expects. For
    /// example in Polkadot, `1` DOT equals `10_000_000_000` "Planck".
    ///
    /// When creating transactions, the [`Encode`] implementation of [`Balance`]
    /// will handle that for you automatically, including encoding it to SCALE
    /// [`Compact`].
    ///
    /// # Example
    ///
    /// ```
    /// use gekko::common::*;
    ///
    /// // Balance of 50 DOT.
    /// let balance = BalanceBuilder::new(Currency::Polkadot).balance(50);
    ///
    /// assert_eq!(balance.as_base_unit(), 50 * 10_000_000_000);
    /// ```
    pub fn as_base_unit(&self) -> u128 {
        self.balance
    }
    /// Converts the balance into the specified metric. Returns `None` if the
    /// balance cannot be represented in the specified metric.
    ///
    /// # Example
    ///
    /// ```
    /// use gekko::common::*;
    ///
    /// // Balance of 50 DOT.
    /// let balance = BalanceBuilder::new(Currency::Polkadot)
    ///     .balance(50);
    ///
    /// assert_eq!(balance.as_metric(Metric::Micro), Some(50_000_000));
    /// assert_eq!(balance.as_metric(Metric::Milli), Some(50_000));
    /// assert_eq!(balance.as_metric(Metric::One), Some(50));
    /// // Cannot be represented in kilo.
    /// assert_eq!(balance.as_metric(Metric::Kilo), None);
    /// ```
    pub fn as_metric(&self, metric: Metric) -> Option<u128> {
        Some(convert_metrics(
            Metric::One,
            metric,
            self.balance / self.unit,
        )?)
    }
}

fn convert_metrics(prev_metric: Metric, new_metric: Metric, balance: u128) -> Option<u128> {
    // Converts negative number to positive.
    fn pos(n: i128) -> u128 {
        let n = if n < 0 { n * -1 } else { n };
        n as u128
    }

    let prev_metric = prev_metric as i128;
    let new_metric = new_metric as i128;

    let max = pos(new_metric).max(pos(prev_metric));
    let min = pos(new_metric).min(pos(prev_metric));

    let balance = if new_metric > prev_metric {
        balance / (max / min)
    } else if new_metric < prev_metric {
        balance.saturating_mul(max * min)
    } else {
        balance
    };

    if balance == 0 {
        None
    } else {
        Some(balance)
    }
}

impl Encode for Balance {
    fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
        f(&Compact::from(self.balance).encode())
    }
}

impl Decode for Balance {
    fn decode<I: Input>(_input: &mut I) -> Result<Self, parity_scale_codec::Error> {
        Err("cannot decode Balance. Use OpaqueBalance instead.".into())
    }
}

#[test]
fn balance_builder() {
    let dot: Balance = BalanceBuilder::new(Currency::Polkadot).balance(50_000);

    // Convert DOT to micro-DOT.
    assert_eq!(dot.as_metric(Metric::Micro).unwrap(), 50_000 * 1_000_000);
    assert_eq!(dot.as_metric(Metric::Milli).unwrap(), 50_000 * 1_000);
    assert_eq!(dot.as_metric(Metric::One).unwrap(), 50_000);
    assert_eq!(dot.as_metric(Metric::Kilo).unwrap(), 50_000 / 1_000);
    assert_eq!(dot.as_metric(Metric::Mega), None);

    assert_eq!(dot.as_base_unit(), Currency::Polkadot.base_unit() * 50_000);
}

// TODO: Add convenience handlers for DOT/KSM.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[rustfmt::skip]
pub enum Metric {
    Peta  =  1_000_000_000_000_000,
    Tera  =  1_000_000_000_000,
    Giga  =  1_000_000_000,
    Mega  =  1_000_000,
    Kilo  =  1_000,
    One  =  1,
    Milli = -1_000,
    Micro = -1_000_000,
    Nano  = -1_000_000_000,
    Pico  = -1_000_000_000_000,
    Femto = -1_000_000_000_000_000,
}

pub struct KeyPairBuilder<T>(std::marker::PhantomData<T>);

impl<T: Pair> KeyPairBuilder<T> {
    pub fn generate() -> (T, T::Seed) {
        T::generate()
    }
    // TODO: Add handler for &[u8]
    pub fn from_seed(seed: &T::Seed) -> T {
        T::from_seed(seed)
    }
    pub fn from_phase(
        phase: &str,
        password: Option<&str>,
    ) -> Result<(T, T::Seed), sp_core::crypto::SecretStringError> {
        T::from_phrase(phase, password)
    }
}

#[derive(Clone)]
pub enum MultiKeyPair {
    Ed25519(Ed25519),
    Sr25519(Sr25519),
    Ecdsa(Ecdsa),
}

impl From<Ed25519> for MultiKeyPair {
    fn from(val: Ed25519) -> Self {
        MultiKeyPair::Ed25519(val)
    }
}

impl From<Sr25519> for MultiKeyPair {
    fn from(val: Sr25519) -> Self {
        MultiKeyPair::Sr25519(val)
    }
}

impl From<Ecdsa> for MultiKeyPair {
    fn from(val: Ecdsa) -> Self {
        MultiKeyPair::Ecdsa(val)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mortality {
    Immortal,
    Mortal(u64, u64, Option<[u8; 32]>),
}

impl Encode for Mortality {
    fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
        // The code within this block was copied from the
        // [Substrate](https://github.com/paritytech/substrate) project, created
        // by Parity Technologies. The copied code is slightly modified. The
        // author of this library takes no credit for the copied code and fully
        // complies with the license of the copied code.
        //
        // Copyright (C) 2017-2021 Parity Technologies (UK) Ltd.
        // SPDX-License-Identifier: Apache-2.0

        let mut enc = Vec::with_capacity(2);

        match self {
            Self::Immortal => enc.push(0),
            Self::Mortal(period, phase, _) => {
                let quantize_factor = (*period as u64 >> 12).max(1);
                let encoded = (period.trailing_zeros() - 1).max(1).min(15) as u16
                    | ((phase / quantize_factor) << 4) as u16;
                encoded.encode_to(&mut enc);
            }
        }

        f(&enc)
    }
}

impl Decode for Mortality {
    fn decode<I: Input>(input: &mut I) -> Result<Self, parity_scale_codec::Error> {
        // The code within this block was copied from the
        // [Substrate](https://github.com/paritytech/substrate) project, created
        // by Parity Technologies. The copied code is slightly modified. The
        // author of this library takes no credit for the copied code and fully
        // complies with the license of the copied code.
        //
        // Copyright (C) 2017-2021 Parity Technologies (UK) Ltd.
        // SPDX-License-Identifier: Apache-2.0

        let first = input.read_byte()?;
        if first == 0 {
            Ok(Self::Immortal)
        } else {
            let encoded = first as u64 + ((input.read_byte()? as u64) << 8);
            let period = 2 << (encoded % (1 << 4));
            let quantize_factor = (period >> 12).max(1);
            let phase = (encoded >> 4) * quantize_factor;
            if period >= 4 && phase < period {
                Ok(Self::Mortal(period, phase, None))
            } else {
                Err("Invalid period and phase".into())
            }
        }
    }
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
pub enum MultiSignature {
    Ed25519(sp_core::ed25519::Signature),
    Sr25519(sp_core::sr25519::Signature),
    Ecdsa(sp_core::ecdsa::Signature),
}

impl From<sp_core::ed25519::Signature> for MultiSignature {
    fn from(val: sp_core::ed25519::Signature) -> Self {
        MultiSignature::Ed25519(val)
    }
}

impl From<sp_core::sr25519::Signature> for MultiSignature {
    fn from(val: sp_core::sr25519::Signature) -> Self {
        MultiSignature::Sr25519(val)
    }
}

impl From<sp_core::ecdsa::Signature> for MultiSignature {
    fn from(val: sp_core::ecdsa::Signature) -> Self {
        MultiSignature::Ecdsa(val)
    }
}

// TODO: Implement MultiAddress.
pub struct MultiAddress;

/// An opaque 32-byte identifier of an on-chain account.
///
/// Usually contains the public key (or its hash in case of ECDSA). This is a
/// simpler implementation of [Substrates
/// `AccountId32`](sp_core::crypto::AccountId32) with some convenience methods.
/// You also use that one instead or convert into it (or from it).
///
/// ```
/// use gekko::common::*;
/// use gekko::common::sp_core::crypto::AccountId32;
///
/// let account_id =
///     AccountId::from_ss58_address("D12RroVkrWavttGJ1g3iHNmDa68kyMsSeXvoZ1xPm8828kk")
///     .unwrap();
///
/// // Convert this type into Substrates `AccountId32`
/// let sub: AccountId32 = account_id.into();
///
/// // Convert it back into the native type.
/// let account_id: AccountId = sub.into();
/// ```
///
/// **Note**: This type should only be used to encode transactions, not decode
/// those. Officially, Kusama and Polkadot support multiple cryptographic
/// identifiers and [`MultiAddress`] should therefore be used for decoding.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AccountId([u8; 32]);

// TODO: Consider adding hex handler.
impl AccountId {
    /// Creates a new account identifier from a byte array.
    pub fn new(bytes: [u8; 32]) -> Self {
        AccountId(bytes)
    }
    /// Creates a new account identifier from a SS58 encoded string.
    ///
    /// # Example
    ///
    /// ```
    /// use gekko::common::*;
    ///
    /// let account_id =
    ///     AccountId::from_ss58_address("D12RroVkrWavttGJ1g3iHNmDa68kyMsSeXvoZ1xPm8828kk")
    ///         .unwrap();
    /// ```
    pub fn from_ss58_address(addr: &str) -> Result<Self, ()> {
        let (account, _) = Self::from_ss58check_with_version(addr).unwrap();
        Ok(account)
    }
    /// Creates a new account identifier from a SS58 encoded string and returns
    /// the identified SS58 Address format.
    ///
    /// # Example
    /// ```
    /// use gekko::common::*;
    /// use gekko::common::sp_core::crypto::Ss58AddressFormat;
    ///
    /// let (account_id, version) =
    ///     AccountId::from_ss58_address_with_version("D12RroVkrWavttGJ1g3iHNmDa68kyMsSeXvoZ1xPm8828kk")
    ///         .unwrap();
    ///
    /// assert_eq!(version, Ss58AddressFormat::KusamaAccount);
    /// ```
    pub fn from_ss58_address_with_version(addr: &str) -> Result<(Self, Ss58AddressFormat), ()> {
        let (account, format) = Self::from_ss58check_with_version(addr).unwrap();
        Ok((account, format))
    }
    /// Returns the SS58 encoded representation of the account identifiers,
    /// based on the specified format.
    pub fn to_ss58_address(&self, format: Ss58AddressFormat) -> String {
        self.to_ss58check_with_version(format)
    }
    /// Returns the underlying byte array of the account identifier. Usually the
    /// public key of the account.
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }
}

impl From<AccountId> for AccountId32 {
    fn from(val: AccountId) -> Self {
        AccountId32::new(val.to_bytes())
    }
}

impl From<AccountId32> for AccountId {
    fn from(val: AccountId32) -> Self {
        AccountId::new(*val.as_ref())
    }
}

impl Encode for AccountId {
    fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
        let mut buffer = [0; 33];

        // The first byte is 0, which represents index 0 of Substrates
        // `sp_runtime::MultiAddress`, i.e. `AccountId` (pubkey).
        buffer[1..].copy_from_slice(&self.0);

        f(&buffer)
    }
}

impl Decode for AccountId {
    fn decode<I: Input>(input: &mut I) -> Result<Self, parity_scale_codec::Error> {
        let mut buffer = [0; 32];
        let idx = input.read_byte()?;
        if idx != 0 {
            return Err("Invalid enum index of AccountId (pubkey), expected 0".into());
        }

        input.read(&mut buffer)?;

        Ok(AccountId(buffer))
    }
}

impl Ss58Codec for AccountId {}

impl AsRef<[u8]> for AccountId {
    fn as_ref(&self) -> &[u8] {
        &self.0[..]
    }
}

impl AsMut<[u8]> for AccountId {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0[..]
    }
}

impl From<sp_core::sr25519::Public> for AccountId {
    fn from(val: sp_core::sr25519::Public) -> Self {
        AccountId(val.0)
    }
}

impl From<sp_core::ed25519::Public> for AccountId {
    fn from(val: sp_core::ed25519::Public) -> Self {
        AccountId(val.0)
    }
}

impl From<MultiKeyPair> for AccountId {
    fn from(val: MultiKeyPair) -> Self {
        match val {
            MultiKeyPair::Ed25519(pair) => pair.public().into(),
            MultiKeyPair::Sr25519(pair) => pair.public().into(),
            MultiKeyPair::Ecdsa(_) => unimplemented!(),
        }
    }
}
