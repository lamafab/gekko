// INFO: The earliest metadata versions are available in the substrate repo at commit: a31c01b398d958ccf0a24d8c1c11fb073df66212

#[macro_use]
extern crate serde;
#[macro_use]
extern crate parity_scale_codec;

use parity_scale_codec::{Decode, Error as ScaleError};
use serde_json::Error as SerdeJsonError;

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    ParseJsonRpcMetadata(SerdeJsonError),
    ParseHexMetadata(hex::FromHexError),
    ParseRawMetadata(ScaleError),
}

#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub result: String,
}

// Convenience function for parsing the Json RPC response returned by `state_getMetadata`. Must fit the [`JsonRpcResponse`] structure.
pub fn parse_jsonrpc_metadata<T: AsRef<[u8]>>(json: T) -> Result<MetadataVersion> {
    let resp = serde_json::from_slice::<JsonRpcResponse>(json.as_ref())
        .map_err(|err| Error::ParseJsonRpcMetadata(err))?;

    parse_hex_metadata(resp.result.as_bytes())
}

// Convenience function for parsing the metadata from a HEX representation, as returned by `state_getMetadata`.
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

pub fn parse_raw_metadata<T: AsRef<[u8]>>(raw: T) -> Result<MetadataVersion> {
    let raw = raw.as_ref();

    // Remove the magic number before decoding, if it exists.
    // From the substrate docs:
    // > "The hex blob that is returned by the JSON-RPCs state_getMetadata method starts with a hard-coded
    // > magic number, 0x6d657461, which represents "meta" in plain text."
    let mut slice = if raw.starts_with(b"meta") {
        raw[4..].as_ref()
    } else {
        raw
    };

    MetadataVersion::decode(&mut slice).map_err(|err| Error::ParseRawMetadata(err))
}

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

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct MetadataV13 {
    modules: Vec<ModuleMetadata>,
    extrinsics: ExtrinsicMetadata,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct ModuleMetadata {
    pub name: String,
    pub storage: Option<StorageMetadata>,
    pub calls: Option<Vec<FunctionMetadata>>,
    pub event: Option<Vec<EventMetadata>>,
    pub constants: Vec<ModuleConstantMetadata>,
    pub errors: Vec<ErrorMetadata>,
    pub index: u8,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct StorageMetadata {
    prefix: String,
    entries: Vec<StorageEntryMetadata>,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct StorageEntryMetadata {
    name: String,
    modifier: StorageEntryModifier,
    ty: StorageEntryType,
    default: Vec<u8>,
    documentation: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub enum StorageEntryModifier {
    Optional,
    Default,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub enum StorageEntryType {
    Plain(String),
    Map {
        hasher: StorageHasher,
        key: String,
        value: String,
        unused: bool,
    },
    DoubleMap {
        hasher: StorageHasher,
        key1: String,
        key2: String,
        value: String,
        key2_hasher: StorageHasher,
    },
    NMap {
        keys: String,
        hashers: Vec<StorageHasher>,
        value: String,
    },
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub enum StorageHasher {
    Blake2_128,
    Blake2_256,
    Blake2_128Concat,
    Twox128,
    Twox256,
    Twox64Concat,
    Identity,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct FunctionMetadata {
    pub name: String,
    pub arguments: Vec<FunctionArgumentMetadata>,
    pub documentation: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct FunctionArgumentMetadata {
    pub name: String,
    pub ty: String,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct EventMetadata {
    pub name: String,
    pub arguments: Vec<String>,
    pub documentation: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct ModuleConstantMetadata {
    pub name: String,
    pub ty: String,
    pub value: Vec<u8>,
    pub documentation: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct ErrorMetadata {
    pub name: String,
    pub documentation: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct ExtrinsicMetadata {
    pub version: u8,
    pub signed_extensions: Vec<String>,
}

#[test]
fn parse_file() {
    use std::fs::read_to_string;

    let content = read_to_string("metadata_sample/metadata_polkadot_sv_9050_tv_7.json").unwrap();
    let _ = parse_jsonrpc_metadata(content).unwrap();
}
