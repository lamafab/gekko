use crate::{ExtrinsicInfo, ModuleMetadataExt, Result};

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct MetadataV13 {
    pub modules: Vec<ModuleMetadata>,
    pub extrinsics: ExtrinsicMetadata,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct ModuleMetadata {
    pub name: String,
    pub storage: Option<StorageMetadata>,
    pub calls: Option<Vec<FunctionMetadata>>,
    pub events: Option<Vec<EventMetadata>>,
    pub constants: Vec<ModuleConstantMetadata>,
    pub errors: Vec<ErrorMetadata>,
    pub index: u8,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct StorageMetadata {
    pub prefix: String,
    pub entries: Vec<StorageEntryMetadata>,
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct StorageEntryMetadata {
    pub name: String,
    pub modifier: StorageEntryModifier,
    pub ty: StorageEntryType,
    pub default: Vec<u8>,
    pub documentation: Vec<String>,
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

impl ModuleMetadataExt for MetadataV13 {
    fn modules_extrinsics<'a>(&'a self) -> Result<Vec<ExtrinsicInfo<'a>>> {
        let infos = self
            .modules
            .iter()
            .enumerate()
            .map(|(module_id, mod_meta)| {
                mod_meta
                    .calls
                    .as_ref()
                    .map(|funcs_meta| {
                        funcs_meta
                            .iter()
                            .enumerate()
                            .map(|(dispatch_id, func_meta)| ExtrinsicInfo {
                                module_id: module_id,
                                dispatch_id: dispatch_id,
                                name: func_meta.name.as_str(),
                                args: func_meta
                                    .arguments
                                    .iter()
                                    .map(|arg_meta| (arg_meta.name.as_str(), arg_meta.ty.as_str()))
                                    .collect(),
                                documentation: func_meta
                                    .documentation
                                    .iter()
                                    .map(|s| s.as_str())
                                    .collect(),
                            })
                            .collect()
                    })
                    .unwrap_or(vec![])
            })
            .flatten()
            .collect();

        Ok(infos)
    }
}
