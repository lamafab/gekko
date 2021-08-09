// # COPYRIGHT DISCLAIMER
//
// Large part of the code visible in this file was copied from the
// [Substrate](https://github.com/paritytech/substrate) project, developed by
// [Parity Technologies](https://www.parity.io/) and licensed under the [Apache
// License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0). The copied
// work was modified by the author of this library. The author of this library
// takes no credit for the copied work and fully complies with the Apache
// License, Version 2.0.
//
// # LICENSE OF THE COPIED WORK
//
// This file is part of Substrate.

// Copyright (C) 2017-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::Result;
use base58::{FromBase58, ToBase58};
use blake2_rfc::blake2b::Blake2b;

pub trait Ss58Codec: Sized + AsMut<[u8]> + AsRef<[u8]> + Default {
	/// Converts the SS58 encoded string into the key and returns it.
    fn from_string(s: &str) -> Result<Self> {
        Self::from_string_with_version(s).map(|(r, _)| r)
    }
    /// Like `from_string`, but will return an error if the address format is `Ss58AddressFormat::Custom(_)`.
    fn from_string_reject_unknown(s: &str) -> Result<Self> {
        Self::from_string_with_version(s).and_then(|(r, v)| match v {
            Ss58AddressFormat::Custom(_) => panic!(),
            _ => Ok(r),
        })
    }
	/// Converts the SS58 encoded string into the key. Returns the key and the identified address format.
    fn from_string_with_version(s: &str) -> Result<(Self, Ss58AddressFormat)> {
        const CHECKSUM_LEN: usize = 2;
        let mut res = Self::default();

        // Must decode to our type.
        let body_len = res.as_mut().len();

        let data = s.from_base58().unwrap();
        if data.len() < 2 {
            panic!()
        }
        let (prefix_len, ident) = match data[0] {
            0..=63 => (1, data[0] as u16),
            64..=127 => {
                // weird bit manipulation owing to the combination of LE encoding and missing two bits
                // from the left.
                // d[0] d[1] are: 01aaaaaa bbcccccc
                // they make the LE-encoded 16-bit value: aaaaaabb 00cccccc
                // so the lower byte is formed of aaaaaabb and the higher byte is 00cccccc
                let lower = (data[0] << 2) | (data[1] >> 6);
                let upper = data[1] & 0b00111111;
                (2, (lower as u16) | ((upper as u16) << 8))
            }
            _ => panic!(),
        };

        if data.len() != prefix_len + body_len + CHECKSUM_LEN {
            panic!()
        }
        let format = ident.into();

        let hash = ss58hash(&data[0..body_len + prefix_len]);
        let checksum = &hash.as_bytes()[0..CHECKSUM_LEN];
        if data[body_len + prefix_len..body_len + prefix_len + CHECKSUM_LEN] != *checksum {
            // Invalid checksum.
            panic!()
        }

        res.as_mut()
            .copy_from_slice(&data[prefix_len..body_len + prefix_len]);
        Ok((res, format))
    }
	/// Returns the SS58 encoded string of the key.
    fn to_string_with_version(&self, version: Ss58AddressFormat) -> String {
        // We mask out the upper two bits of the ident - SS58 Prefix currently only supports 14-bits
        let ident: u16 = u16::from(version) & 0b0011_1111_1111_1111;
        let mut v = match ident {
            0..=63 => vec![ident as u8],
            64..=16_383 => {
                // upper six bits of the lower byte(!)
                let first = ((ident & 0b0000_0000_1111_1100) as u8) >> 2;
                // lower two bits of the lower byte in the high pos,
                // lower bits of the upper byte in the low pos
                let second = ((ident >> 8) as u8) | ((ident & 0b0000_0000_0000_0011) as u8) << 6;
                vec![first | 0b01000000, second]
            }
            _ => unreachable!("masked out the upper two bits; qed"),
        };
        v.extend(self.as_ref());
        let r = ss58hash(&v);
        v.extend(&r.as_bytes()[0..2]);
        v.to_base58()
    }
}

const PREFIX: &[u8] = b"SS58PRE";

fn ss58hash(data: &[u8]) -> blake2_rfc::blake2b::Blake2bResult {
    let mut context = Blake2b::new(64);
    context.update(PREFIX);
    context.update(data);
    context.finalize()
}

macro_rules! ss58_address_format {
	( $( $identifier:tt => ($number:expr, $name:expr, $desc:tt) )* ) => (
		/// A known address (sub)format/network ID for SS58.
		#[derive(Debug, Clone, Copy, PartialEq, Eq)]
		pub enum Ss58AddressFormat {
			$(#[doc = $desc] $identifier),*,
			/// Use a manually provided numeric value as a standard identifier
			Custom(u16),
		}

		impl From<Ss58AddressFormat> for u16 {
			fn from(x: Ss58AddressFormat) -> u16 {
				match x {
					$(Ss58AddressFormat::$identifier => $number),*,
					Ss58AddressFormat::Custom(n) => n,
				}
			}
		}

		impl From<u16> for Ss58AddressFormat {
			fn from(x: u16) -> Ss58AddressFormat {
				match x {
					$($number => Ss58AddressFormat::$identifier),*,
					_ => Ss58AddressFormat::Custom(x),
				}
			}
		}

		impl std::fmt::Display for Ss58AddressFormat {
			fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
				match self {
					$(
						Ss58AddressFormat::$identifier => write!(f, "{}", $name),
					)*
					Ss58AddressFormat::Custom(x) => write!(f, "{}", x),
				}

			}
		}

	)
}

ss58_address_format!(
    PolkadotAccount =>
        (0, "polkadot", "Polkadot Relay-chain, standard account (*25519).")
    BareSr25519 =>
        (1, "sr25519", "Bare 32-bit Schnorr/Ristretto 25519 (S/R 25519) key.")
    KusamaAccount =>
        (2, "kusama", "Kusama Relay-chain, standard account (*25519).")
    BareEd25519 =>
        (3, "ed25519", "Bare 32-bit Edwards Ed25519 key.")
    KatalChainAccount =>
        (4, "katalchain", "Katal Chain, standard account (*25519).")
    PlasmAccount =>
        (5, "plasm", "Plasm Network, standard account (*25519).")
    BifrostAccount =>
        (6, "bifrost", "Bifrost mainnet, direct checksum, standard account (*25519).")
    EdgewareAccount =>
        (7, "edgeware", "Edgeware mainnet, standard account (*25519).")
    KaruraAccount =>
        (8, "karura", "Acala Karura canary network, standard account (*25519).")
    ReynoldsAccount =>
        (9, "reynolds", "Laminar Reynolds canary network, standard account (*25519).")
    AcalaAccount =>
        (10, "acala", "Acala mainnet, standard account (*25519).")
    LaminarAccount =>
        (11, "laminar", "Laminar mainnet, standard account (*25519).")
    PolymathAccount =>
        (12, "polymath", "Polymath network, standard account (*25519).")
    SubstraTeeAccount =>
        (13, "substratee", "Any SubstraTEE off-chain network private account (*25519).")
    TotemAccount =>
        (14, "totem", "Any Totem Live Accounting network standard account (*25519).")
    SynesthesiaAccount =>
        (15, "synesthesia", "Synesthesia mainnet, standard account (*25519).")
    KulupuAccount =>
        (16, "kulupu", "Kulupu mainnet, standard account (*25519).")
    DarkAccount =>
        (17, "dark", "Dark mainnet, standard account (*25519).")
    DarwiniaAccount =>
        (18, "darwinia", "Darwinia Chain mainnet, standard account (*25519).")
    GeekAccount =>
        (19, "geek", "GeekCash mainnet, standard account (*25519).")
    StafiAccount =>
        (20, "stafi", "Stafi mainnet, standard account (*25519).")
    DockTestAccount =>
        (21, "dock-testnet", "Dock testnet, standard account (*25519).")
    DockMainAccount =>
        (22, "dock-mainnet", "Dock mainnet, standard account (*25519).")
    ShiftNrg =>
        (23, "shift", "ShiftNrg mainnet, standard account (*25519).")
    ZeroAccount =>
        (24, "zero", "ZERO mainnet, standard account (*25519).")
    AlphavilleAccount =>
        (25, "alphaville", "ZERO testnet, standard account (*25519).")
    JupiterAccount =>
        (26, "jupiter", "Jupiter testnet, standard account (*25519).")
    SubsocialAccount =>
        (28, "subsocial", "Subsocial network, standard account (*25519).")
    DhiwayAccount =>
        (29, "cord", "Dhiway CORD network, standard account (*25519).")
    PhalaAccount =>
        (30, "phala", "Phala Network, standard account (*25519).")
    LitentryAccount =>
        (31, "litentry", "Litentry Network, standard account (*25519).")
    RobonomicsAccount =>
        (32, "robonomics", "Any Robonomics network standard account (*25519).")
    DataHighwayAccount =>
        (33, "datahighway", "DataHighway mainnet, standard account (*25519).")
    AresAccount =>
        (34, "ares", "Ares Protocol, standard account (*25519).")
    ValiuAccount =>
        (35, "vln", "Valiu Liquidity Network mainnet, standard account (*25519).")
    CentrifugeAccount =>
        (36, "centrifuge", "Centrifuge Chain mainnet, standard account (*25519).")
    NodleAccount =>
        (37, "nodle", "Nodle Chain mainnet, standard account (*25519).")
    KiltAccount =>
        (38, "kilt", "KILT Chain mainnet, standard account (*25519).")
    PolimecAccount =>
        (41, "poli", "Polimec Chain mainnet, standard account (*25519).")
    SubstrateAccount =>
        (42, "substrate", "Any Substrate network, standard account (*25519).")
    BareSecp256k1 =>
        (43, "secp256k1", "Bare ECDSA SECP256k1 key.")
    ChainXAccount =>
        (44, "chainx", "ChainX mainnet, standard account (*25519).")
    UniartsAccount =>
        (45, "uniarts", "UniArts Chain mainnet, standard account (*25519).")
    Reserved46 =>
        (46, "reserved46", "Reserved for future use (46).")
    Reserved47 =>
        (47, "reserved47", "Reserved for future use (47).")
    NeatcoinAccount =>
        (48, "neatcoin", "Neatcoin mainnet, standard account (*25519).")
    HydraDXAccount =>
        (63, "hydradx", "HydraDX standard account (*25519).")
    AventusAccount =>
        (65, "aventus", "Aventus Chain mainnet, standard account (*25519).")
    CrustAccount =>
        (66, "crust", "Crust Network, standard account (*25519).")
    EquilibriumAccount =>
        (67, "equilibrium", "Equilibrium Network, standard account (*25519).")
    SoraAccount =>
        (69, "sora", "SORA Network, standard account (*25519).")
    ZeitgeistAccount =>
        (73, "zeitgeist", "Zeitgeist network, standard account (*25519).")
    MantaAccount =>
        (77, "manta", "Manta Network, standard account (*25519).")
    CalamariAccount =>
        (78, "calamari", "Manta Canary Network, standard account (*25519).")
    PolkaSmith =>
        (98, "polkasmith", "PolkaSmith Canary Network, standard account (*25519).")
    PolkaFoundry =>
        (99, "polkafoundry", "PolkaFoundry Network, standard account (*25519).")
    OriginTrailAccount =>
        (101, "origintrail-parachain", "OriginTrail Parachain, ethereumm account (ECDSA).")
    HeikoAccount =>
        (110, "heiko", "Heiko, session key (*25519).")
    ParallelAccount =>
        (172, "parallel", "Parallel, session key (*25519).")
    SocialAccount =>
        (252, "social-network", "Social Network, standard account (*25519).")
    Moonbeam =>
        (1284, "moonbeam", "Moonbeam, session key (*25519).")
    Moonriver =>
        (1285, "moonriver", "Moonriver, session key (*25519).")
    BasiliskAccount =>
        (10041, "basilisk", "Basilisk standard account (*25519).")

    // Note: 16384 and above are reserved.
);
