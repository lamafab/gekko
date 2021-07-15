#[macro_use]
extern crate serde;
#[macro_use]
extern crate parity_scale_codec;

use parity_scale_codec::Decode;
use serde_json::Error as SerdeJsonError;

type Result<T> = std::result::Result<T, Error>;

pub enum Error {
    ParseJsonRpcMetadata(SerdeJsonError),
    ParseHexMetadata(hex::FromHexError),
}

// INFO: The earliest metadata versions are available in the substrate repo at commit: a31c01b398d958ccf0a24d8c1c11fb073df66212

// The magic number that is prefixed in the runtime metadata returned by JSON-RPC `state_getMetadata`. 'meta' = 0x6d657461.
const MAGIC_NUMBER: &'static str = "meta";

#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub result: String,
}

// Convenience function for parsing the Json RPC response returned by `state_getMetadata`. Must fit the [`JsonRpcResponse`] structure.
pub fn parse_jsonrpc_metadata<T: AsRef<[u8]>>(json: T) -> Result<MetadataVersion> {
    let resp = serde_json::from_slice::<JsonRpcResponse>(json.as_ref())
        .map_err(|err| Error::ParseJsonRpcMetadata(err))?;

    parse_raw_metadata(resp.result.as_bytes())
}

// Convenience function for parsing the metadata from a HEX representation, as returned by `state_getMetadata`.
pub fn parse_hex_metadata<T: AsRef<[u8]>>(hex: T) -> Result<MetadataVersion> {
    parse_raw_metadata(hex::decode(hex).map_err(|err| Error::ParseHexMetadata(err))?)
}

pub fn parse_raw_metadata<T: AsRef<[u8]>>(mut raw: T) -> Result<MetadataVersion> {
    Ok(MetadataVersion::decode(&mut raw.as_ref()[MAGIC_NUMBER.len()..].as_ref()).unwrap())
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub enum MetadataVersion {
    V01,
    V02,
    V03,
    V04,
    V05,
    V06,
    V07,
    V08,
    V09,
    V10,
    V11,
    V12,
    V13(MetadataV13),
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct MetadataV13 {
    // The magic number
    prefix: String,
    modules: Vec<ModuleMetadata>,
    extrinsics: ExtrinsicMetadata,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct ModuleMetadata {
    pub name: String,
    pub storage: Option<StorageMetadata>,
    pub calls: Option<Vec<FunctionMetadata>>,
    pub event: Option<Vec<EventMetadata>>,
    pub constants: ModuleConstantMetadata,
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
    pub documentation: String,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct ExtrinsicMetadata {
    pub version: u8,
    pub signed_extensions: Vec<String>,
}
