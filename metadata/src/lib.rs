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

mod version;

pub struct ExtrinsicInfo<'a> {
    pub module_id: usize,
    pub dispatch_id: usize,
    pub name: &'a str,
    pub args: Vec<(&'a str, &'a str)>,
    pub documentation: Vec<&'a str>,
}

pub trait ModuleMetadataExt {
    fn modules_extrinsics<'a>(&'a self) -> Vec<ExtrinsicInfo<'a>>;
    fn find_module_extrinsic<'a>(
        &'a self,
        method: &str,
        extrinsic: &str,
    ) -> Result<Option<ExtrinsicInfo<'a>>>;
}

#[derive(Debug)]
pub enum Error {
    ParseJsonRpcMetadata(SerdeJsonError),
    ParseHexMetadata(hex::FromHexError),
    ParseRawMetadata(ScaleError),
    InvalidMetadataVersion,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub result: String,
}

// Convenience function for parsing the Json RPC response returned by
// `state_getMetadata`. Must fit the [`JsonRpcResponse`] structure.
pub fn parse_jsonrpc_metadata<T: AsRef<[u8]>>(json: T) -> Result<MetadataVersion> {
    let resp = serde_json::from_slice::<JsonRpcResponse>(json.as_ref())
        .map_err(|err| Error::ParseJsonRpcMetadata(err))?;

    parse_hex_metadata(resp.result.as_bytes())
}

// Convenience function for parsing the metadata from a HEX representation, as
// returned by `state_getMetadata`.
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
    pub fn into_inner(self) -> impl ModuleMetadataExt {
        match self {
            MetadataVersion::V13(m) => m,
            _ => panic!(),
        }
    }
}

#[test]
fn parse_file() {
    use std::fs::read_to_string;

    let content = read_to_string("dumps/metadata_polkadot_9050.json").unwrap();
    let res = parse_jsonrpc_metadata(content).unwrap();

    let data = match res {
        MetadataVersion::V13(data) => data,
        _ => panic!(),
    };

    for m in data.modules {
        println!("> {}", m.name);
        m.calls.map(|calls| {
            for c in calls {
                println!("  > {}", c.name);
                for arg in c.arguments {
                    println!("    > {}: {}", arg.name, arg.ty);
                }
            }
        });
    }
}
