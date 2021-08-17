use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use tokio::time::{sleep, Duration};

const BLOCK_HASH_LIMIT: u64 = 50;
const TIMEOUT: u64 = 10;

type Result<T> = std::result::Result<T, anyhow::Error>;

// TODO
struct Config;

pub struct CollectorConfig {
    chain_name: String,
}

/// Handler to save the collected information to disk.
struct Filesystem {
    config: CollectorConfig,
}

impl Filesystem {
    const LOCATION: &'static str = "/var/lib/metadata_collector";
    const STATE: &'static str = ".collection_state";

    fn new(config: CollectorConfig) -> Self {
        Filesystem { config: config }
    }
    fn path(&self) -> String {
        format!("{}/{}/", Self::LOCATION, self.config.chain_name)
    }
    fn save_runtime_metadata(&self, version: RuntimeVersion, metadata: MetadataHex) -> Result<()> {
        // Save information about the runtime version.
        let mut file = File::create(&format!(
            "{}version_{}_{}.json",
            self.path(),
            version.spec_name,
            version.spec_version
        ))?;

        file.write_all(serde_json::to_string(&version)?.as_bytes())?;
        file.sync_all()?;

        // Save the metadata of the runtime.
        let mut file = File::create(&format!(
            "{}metadata_{}_{}.hex",
            self.path(),
            version.spec_name,
            version.spec_version
        ))?;

        file.write_all(metadata.0.as_bytes())?;
        file.sync_all()?;

        Ok(())
    }
    fn read_last_state(&self) -> Result<Option<LatestInfo>> {
        let path = format!("{}{}", self.path(), Self::STATE);

        if !Path::new(&path).exists() {
            return Ok(None);
        }

        let mut file = File::open(&path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        Ok(Some(serde_json::from_str(&contents)?))
    }
    fn track_latest_state(&self, state: &LatestInfo) -> Result<()> {
        let mut file = File::create(&format!("{}{}", self.path(), Self::STATE))?;
        file.write_all(serde_json::to_string_pretty(&state)?.as_bytes())?;
        file.sync_all()?;

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestInfo {
    spec_name: String,
    spec_version: u64,
    last_block: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest<'a, T> {
    pub id: i64,
    pub jsonrpc: &'a str,
    pub method: &'a str,
    pub params: T,
}

impl<'a, T> RpcRequest<'a, T> {
    fn new(method: RpcMethod, params: T) -> Self {
        RpcRequest {
            id: 1,
            jsonrpc: "2.0",
            method: method.as_str(),
            params: params,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse<T> {
    pub jsonrpc: String,
    pub result: T,
    pub id: i64,
}

#[derive(Debug, Clone, Copy)]
enum RpcMethod {
    Header,
    BlockHash,
    RuntimeVersion,
    Metadata,
}

impl RpcMethod {
    fn as_str(&self) -> &'static str {
        match self {
            Header => "chain_getHeader",
            BlockHash => "chain_getBlockHash",
            RuntimeVersion => "state_getRuntimeVersion",
            Metadata => "state_getMetadata",
        }
    }
}

pub async fn local() -> Result<()> {
    let mut latest = latest_block().await?;

    let mut current_block = 0;
    loop {
        let to = latest.min(BLOCK_HASH_LIMIT);
        let range = (current_block..=to).collect();

        let header_hashes = get::<Vec<u64>, Vec<String>>(RpcMethod::BlockHash, range).await?;
        for hash in header_hashes {
            let version = get::<_, RuntimeVersion>(RpcMethod::RuntimeVersion, hash.clone()).await?;
            let metadata = get::<_, MetadataHex>(RpcMethod::RuntimeVersion, hash).await?;
        }

        current_block = to + 1;

        if latest < current_block {
            sleep(Duration::from_secs(TIMEOUT)).await;
            latest = latest_block().await?;
        }
    }
}

/// Convenience function for fetching the latest block number.
async fn latest_block() -> Result<u64> {
    get::<Option<()>, Header>(RpcMethod::Header, None)
        .await?
        .number
        .parse()
        .map_err(|err| anyhow::Error::from(err))
}

/// Response when calling `chain_getHeader`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    pub digest: serde_json::Value,
    #[serde(rename = "extrinsicsRoot")]
    pub extrinsics_root: String,
    pub number: String,
    #[serde(rename = "parentHash")]
    pub parent_hash: String,
    #[serde(rename = "stateRoot")]
    pub state_root: String,
}

/// Response when calling `state_getRuntimeVersion`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeVersion {
    pub apis: Vec<(String, i64)>,
    #[serde(rename = "authoringVersion")]
    pub authoring_version: i64,
    #[serde(rename = "implName")]
    pub impl_name: String,
    #[serde(rename = "implVersion")]
    pub impl_version: i64,
    #[serde(rename = "specName")]
    pub spec_name: String,
    #[serde(rename = "specVersion")]
    pub spec_version: i64,
}

/// Response when calling `state_getMetadata`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MetadataHex(String);

/// Convenience function for executing a RPC call.
async fn get<B: Serialize, R: DeserializeOwned>(method: RpcMethod, body: B) -> Result<R> {
    Client::new()
        .post(method.as_str())
        .json(&RpcRequest::new(method, body))
        .send()
        .await?
        .json::<RpcResponse<R>>()
        .await
        .map(|res| res.result)
        .map_err(|err| err.into())
}
