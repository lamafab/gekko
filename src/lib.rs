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
    modules: Vec<()>,
    extrinsics: (),
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

pub struct StorageMetadata {}
pub struct FunctionMetadata {}
pub struct EventMetadata {}
pub struct ModuleConstantMetadata {}
pub struct ErrorMetadata {}
