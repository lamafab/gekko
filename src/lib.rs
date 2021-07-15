// INFO: The earliest metadata versions are available in the substrate repo at commit: a31c01b398d958ccf0a24d8c1c11fb073df66212

// The magic number that is prefixed in the runtime metadata returned by JSON-RPC `state_getMetadata`. 'meta' = 0x6d657461.
const MAGIC_NUMBER: &'static str = "meta";

enum MetadataVersion {
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

struct MetadataV13 {
    // The magic number
    prefix: String,
    modules: Vec<ModuleMetadata>,
    extrinsics: ExtrinsicMetadata,
}

pub struct ModuleMetadata {
    pub name: String,
    pub storage: Option<StorageMetadata>,
    pub calls: Option<Vec<FunctionMetadata>>,
    pub event: Option<Vec<EventMetadata>>,
    pub constants: ModuleConstantMetadata,
    pub errors: Vec<ErrorMetadata>,
    pub index: u8,
}

pub struct StorageMetadata {
    prefix: String,
    entries: Vec<StorageEntryMetadata>,
}

pub struct StorageEntryMetadata {
    name: String,
    modifier: StorageEntryModifier,
    ty: StorageEntryType,
    default: Vec<u8>,
    documentation: Vec<String>,
}

pub enum StorageEntryModifier {
    Optional,
    Default,
}

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

pub enum StorageHasher {
    Blake2_128,
    Blake2_256,
    Blake2_128Concat,
    Twox128,
    Twox256,
    Twox64Concat,
    Identity,
}

pub struct FunctionMetadata {
    pub name: String,
    pub arguments: Vec<FunctionArgumentMetadata>,
    pub documentation: Vec<String>,
}

pub struct FunctionArgumentMetadata {
    pub name: String,
    pub ty: String,
}

pub struct EventMetadata {
    pub name: String,
    pub arguments: Vec<String>,
    pub documentation: Vec<String>,
}

pub struct ModuleConstantMetadata {
    pub name: String,
    pub ty: String,
    pub value: Vec<u8>,
    pub documentation: Vec<String>,
}

pub struct ErrorMetadata {
    pub name: String,
    pub documentation: String,
}

pub struct ExtrinsicMetadata {
    pub version: u8,
    pub signed_extensions: Vec<String>,
}
