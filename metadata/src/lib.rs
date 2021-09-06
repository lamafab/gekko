//! Utilities to parse and process substrate metadata. Can also be enabled in
//! `gekko` with the `"metadata"` feature.
//!
//! # Example
//!
//! ```no_run
//! use gekko_metadata::*;
//!
//! // Parse runtime metadata
//! let content = std::fs::read_to_string("metadata_kusama_9080.hex").unwrap();
//! let data = parse_hex_metadata(content).unwrap().into_inner();
//!
//! // Get information about the extrinsic.
//! let extr = data
//!     .find_module_extrinsic("Balances", "transfer_keep_alive")
//!     .unwrap();
//!
//! assert_eq!(extr.module_id, 4);
//! assert_eq!(extr.dispatch_id, 3);
//! assert_eq!(
//!     extr.args,
//!     vec![
//!         ("dest", "<T::Lookup as StaticLookup>::Source"),
//!         ("value", "Compact<T::Balance>"),
//!     ]
//! );
//! ```

// INFO: The earliest metadata versions are available in the substrate repo at
// commit: a31c01b398d958ccf0a24d8c1c11fb073df66212

#[macro_use]
extern crate serde;
#[macro_use]
extern crate parity_scale_codec;

use self::version::*;
use parity_scale_codec::{Decode, Error as ScaleError};
use serde_json::Error as SerdeJsonError;

type Result<T> = std::result::Result<T, Error>;

pub mod version;

/// Parameters and other information about an individual extrinsic.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExtrinsicInfo<'a> {
    /// The module Id. This is required when encoding the final extrinsic.
    pub module_id: usize,
    /// The dispatch Id. This is required when encoding the final extrinsic.
    pub dispatch_id: usize,
    /// The name of the module.
    pub module_name: &'a str,
    /// The name of the extrinsic.
    pub extrinsic_name: &'a str,
    /// Arguments that must be passed as the extrinsics body. A sequence of
    /// key-value pairs, indicating the name and the type, respectively.
    pub args: Vec<(&'a str, &'a str)>,
    /// Documentation of the extrinsic, as provided by the Substrate metadata.
    pub documentation: Vec<&'a str>,
}

/// Parameters and other information about an individual storage entry.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct StorageInfo<'a> {
    pub module_name: &'a str,
    /// The name of the storage entry.
    pub entry_name: &'a str,
    pub modifier: StorageEntryModifier,
    pub ty: &'a StorageEntryType,
    pub default: Option<&'a [u8]>,
    /// Documentation of the storage entry, as provided by the Substrate metadata.
    pub documentation: &'a [String],
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum StorageEntryModifier {
    Optional,
    Default,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum StorageEntryType {
    Plain(String),
    Map {
        hasher: Option<StorageHasher>,
        key: String,
        value: String,
        unused: Option<bool>,
        is_linked: Option<bool>,
    },
    DoubleMap {
        hasher: Option<StorageHasher>,
        key1: String,
        key2: String,
        value: String,
        key2_hasher: Option<StorageHasher>,
        is_linked: Option<bool>,
    },
    NMap {
        keys: String,
        hashers: Option<Vec<StorageHasher>>,
        value: String,
        is_linked: Option<bool>,
    },
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum StorageHasher {
    Blake2_128,
    Blake2_256,
    Blake2_128Concat,
    Twox128,
    Twox256,
    Twox64Concat,
    Identity,
}

/// An interface to retrieve information about extrinsics on any Substrate
/// metadata version.
pub trait ModuleBuilderExt {
    fn modules_extrinsics<'a>(&'a self) -> Vec<ExtrinsicInfo<'a>>;
    fn find_module_extrinsic<'a>(
        &'a self,
        method: &str,
        extrinsic: &str,
    ) -> Option<ExtrinsicInfo<'a>>;
}

/// An interface to retrieve information about storage entries on any Substrate
/// metadata version.
pub trait StorageBuilderExt {
    fn storage_entries<'a>(&'a self) -> Vec<StorageInfo<'a>>;
    fn find_storage_entries<'a>(&'a self, module: &str, name: &str) -> Option<StorageInfo<'a>>;
}

/// Errors that can occur when parsing Substrate metadata.
#[derive(Debug)]
pub enum Error {
    ParseJsonRpcMetadata(SerdeJsonError),
    ParseHexMetadata(hex::FromHexError),
    ParseRawMetadata(ScaleError),
    InvalidMetadataVersion,
}

/// Helper type when dealing with the Json RPC response returned by
/// Substrates `state_getMetadata`.
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub result: String,
}

/// Convenience function for parsing the Json RPC response returned by Substrates
/// `state_getMetadata`.
///
/// Must fit the [`JsonRpcResponse`] structure.
pub fn parse_jsonrpc_metadata<T: AsRef<[u8]>>(json: T) -> Result<MetadataVersion> {
    let resp = serde_json::from_slice::<JsonRpcResponse>(json.as_ref())
        .map_err(|err| Error::ParseJsonRpcMetadata(err))?;

    parse_hex_metadata(resp.result.as_bytes())
}

/// Convenience function for parsing the metadata from a HEX representation, as
/// returned by `state_getMetadata`.
pub fn parse_hex_metadata<T: AsRef<[u8]>>(hex: T) -> Result<MetadataVersion> {
    let hex = hex.as_ref();

    // The `hex` crate does not handle `0x`...
    let slice = if hex.starts_with(b"0x") {
        hex[2..].as_ref()
    } else {
        hex
    };

    parse_raw_metadata(hex::decode(slice).map_err(|err| Error::ParseHexMetadata(err))?)
}

/// Parse the raw Substrate metadata.
pub fn parse_raw_metadata<T: AsRef<[u8]>>(raw: T) -> Result<MetadataVersion> {
    let raw = raw.as_ref();

    // Remove the magic number before decoding, if it exists. From the substrate
    // docs:
    // > "The hex blob that is returned by the JSON-RPCs state_getMetadata
    // > method starts with a hard-coded magic number, 0x6d657461, which
    // > represents "meta" in plain text."
    let mut slice = if raw.starts_with(b"meta") {
        raw[4..].as_ref()
    } else {
        raw
    };

    MetadataVersion::decode(&mut slice).map_err(|err| Error::ParseRawMetadata(err))
}

/// Identifier of all the available Substrate metadata versions.
#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub enum MetadataVersion {
    V0,
    V1,
    V2,
    V3,
    V4,
    V5,
    V6,
    V7,
    V8,
    V9,
    V10,
    V11,
    V12,
    V13(MetadataV13),
}

impl MetadataVersion {
    /// Consumes the object and returns the inner metadata structure, expecting
    /// the latest version. Results in an error if the version is not the latest.
    pub fn into_latest(self) -> Result<MetadataV13> {
        match self {
            MetadataVersion::V13(data) => Ok(data),
            _ => Err(Error::InvalidMetadataVersion),
        }
    }
    /// Returns the version number as an integer.
    pub fn version_number(&self) -> usize {
        use MetadataVersion::*;

        match self {
            V0 => 0,
            V1 => 1,
            V2 => 2,
            V3 => 3,
            V4 => 4,
            V5 => 5,
            V6 => 6,
            V7 => 7,
            V8 => 8,
            V9 => 9,
            V10 => 10,
            V11 => 11,
            V12 => 12,
            V13(_) => 13,
        }
    }
    pub fn into_inner(self) -> impl ModuleBuilderExt {
        match self {
            MetadataVersion::V13(m) => m,
            _ => panic!(),
        }
    }
}
