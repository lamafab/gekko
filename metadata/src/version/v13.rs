use crate::{ExtrinsicInfo, ModuleBuilderExt, StorageBuilderExt, StorageInfo};

// TODO: Should implement Serialize/Deserialize.
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

#[derive(Debug, Clone, Copy, PartialEq, Encode, Decode)]
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

impl From<StorageEntryType> for crate::StorageEntryType {
    fn from(val: StorageEntryType) -> Self {
        match val {
            StorageEntryType::Plain(s) => crate::StorageEntryType::Plain(s),
            StorageEntryType::Map {
                hasher,
                key,
                value,
                unused,
            } => crate::StorageEntryType::Map {
                hasher: Some(hasher.into()),
                key: key,
                value: value,
                unused: Some(unused),
                is_linked: None,
            },
            StorageEntryType::DoubleMap {
                hasher,
                key1,
                key2,
                value,
                key2_hasher,
            } => crate::StorageEntryType::DoubleMap {
                hasher: Some(hasher.into()),
                key1: key1,
                key2: key2,
                value: value,
                key2_hasher: Some(key2_hasher.into()),
                is_linked: None,
            },
            StorageEntryType::NMap {
                keys,
                hashers,
                value,
            } => crate::StorageEntryType::NMap {
                keys: keys,
                hashers: Some(hashers.into_iter().map(|h| h.into()).collect()),
                value: value,
                is_linked: None,
            },
        }
    }
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

impl From<StorageHasher> for crate::StorageHasher {
    fn from(val: StorageHasher) -> Self {
        match val {
            StorageHasher::Blake2_128 => crate::StorageHasher::Blake2_128,
            StorageHasher::Blake2_256 => crate::StorageHasher::Blake2_256,
            StorageHasher::Blake2_128Concat => crate::StorageHasher::Blake2_128Concat,
            StorageHasher::Twox128 => crate::StorageHasher::Twox128,
            StorageHasher::Twox256 => crate::StorageHasher::Twox256,
            StorageHasher::Twox64Concat => crate::StorageHasher::Twox64Concat,
            StorageHasher::Identity => crate::StorageHasher::Identity,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Encode, Decode)]
pub struct FunctionMetadata {
    pub name: String,
    pub arguments: Vec<FunctionArgumentMetadata>,
    pub documentation: Vec<String>,
}

impl FunctionMetadata {
    pub fn to_extrinsic_info<'a>(
        &'a self,
        module_id: usize,
        dispatch_id: usize,
        module_name: &'a str,
    ) -> ExtrinsicInfo<'a> {
        ExtrinsicInfo {
            module_id: module_id,
            dispatch_id: dispatch_id,
            module_name: module_name,
            extrinsic_name: self.name.as_str(),
            args: self
                .arguments
                .iter()
                .map(|arg_meta| (arg_meta.name.as_str(), arg_meta.ty.as_str()))
                .collect(),
            documentation: self.documentation.iter().map(|s| s.as_str()).collect(),
        }
    }
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

impl ModuleBuilderExt for MetadataV13 {
    fn modules_extrinsics<'a>(&'a self) -> Vec<ExtrinsicInfo<'a>> {
        self.modules
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
                            .map(|(dispatch_id, func_meta)| {
                                func_meta.to_extrinsic_info(
                                    module_id,
                                    dispatch_id,
                                    mod_meta.name.as_str(),
                                )
                            })
                            .collect()
                    })
                    .unwrap_or(vec![])
            })
            .flatten()
            .collect()
    }
    fn find_module_extrinsic<'a>(
        &'a self,
        method: &str,
        extrinsic: &str,
    ) -> Option<ExtrinsicInfo<'a>> {
        self.modules
            .iter()
            .enumerate()
            .find(|(_, mod_meta)| mod_meta.name.as_str() == method)
            .map(|(module_id, mod_meta)| {
                mod_meta.calls.as_ref().map(|funcs_meta| {
                    funcs_meta
                        .iter()
                        .enumerate()
                        .find(|(_, func_meta)| func_meta.name.as_str() == extrinsic)
                        .map(|(dispatch_id, func_meta)| {
                            func_meta.to_extrinsic_info(
                                module_id,
                                dispatch_id,
                                mod_meta.name.as_str(),
                            )
                        })
                })
            })
            .and_then(|res| res?)
    }
}

/*
impl StorageBuilderExt for MetadataV13 {
    fn storage_entries<'a>(&'a self) -> Vec<StorageInfo<'a>> {
        self.modules
            .iter()
            .map(|module| {
                module
                    .storage
                    .map(|storage| {
                        storage
                            .entries
                            .iter()
                            .map(|entry| {
                            StorageInfo {
                                module_name: module.name.as_str(),
                                entry_name: entry.name.as_str(),
                                modifier: entry.modifier,
                                ty: &entry.ty,
                                default: Some(&entry.default),
                                documentation: &entry.documentation,
                            }
                        })
                    })
            })
            .flatten()
            .collect()
    }
    fn find_storage_entries<'a>(&'a self, module: &str, name: &str) -> Option<StorageInfo<'a>> {
        unimplemented!()
    }
}
*/
