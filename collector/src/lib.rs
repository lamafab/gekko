#[macro_use]
extern crate log;

use anyhow::anyhow;
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    collectors: Vec<CollectorConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectorConfig {
    chain_name: String,
    directory: Option<String>,
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
        format!(
            "{}/{}/",
            self.config
                .directory
                .as_ref()
                .map(|dir| dir.as_str())
                .unwrap_or(Self::LOCATION),
            self.config.chain_name
        )
    }
    fn save_runtime_metadata(
        &self,
        version: &RuntimeVersion,
        metadata: &MetadataHex,
    ) -> Result<()> {
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
            Self::Header => "chain_getHeader",
            Self::BlockHash => "chain_getBlockHash",
            Self::RuntimeVersion => "state_getRuntimeVersion",
            Self::Metadata => "state_getMetadata",
        }
    }
}

pub async fn run(config: Config) {
    for collector in config.collectors {
        info!("Starting collector for {}", collector.chain_name);
        tokio::spawn(local(collector));
    }
}

pub async fn local(config: CollectorConfig) -> Result<()> {
    let fs = Filesystem::new(config.clone());

    // Fetch the latest known block number.
    let mut latest = latest_block().await?;

    // Retrieve the last block number from where data should start being fetched from.
    let mut state = fs
        .read_last_state()?
        .map(|mut state| {
            state.last_block += 1;
            state
        })
        .unwrap_or(LatestInfo {
            spec_version: 0,
            last_block: 0,
        });

    loop {
        // Do not skip block 0 when starting at the beginning.
        let from = if state.last_block == 0 {
            state.last_block
        } else {
            state.last_block + 1
        };

        // Set range of block numbers, do not exceed limit.
        let to = latest.min(BLOCK_HASH_LIMIT);
        let range = (from..=to).collect();

        let header_hashes = get::<Vec<u64>, Vec<String>>(RpcMethod::BlockHash, range).await?;
        for hash in header_hashes {
            let version = get::<_, RuntimeVersion>(RpcMethod::RuntimeVersion, hash.clone()).await?;
            let metadata = get::<_, MetadataHex>(RpcMethod::Metadata, hash).await?;

            if version.spec_name != config.chain_name {
                return Err(anyhow!(
                    "Fetching data from the wrong chain, expected {}, got {}",
                    config.chain_name,
                    version.spec_name,
                ));
            }

            if version.spec_version != state.spec_version {
                info!(
                    "Found new runtime version {} at block {}, saving metadata...",
                    version.spec_version, state.last_block
                );

                fs.save_runtime_metadata(&version, &metadata)?;
            } else {
                debug!(
                    "No new version found at block {}, continuing",
                    version.spec_version
                );
            }

            state.last_block += 1;
            state.spec_version = version.spec_version;

            fs.track_latest_state(&state)?;
        }

        //fs.track_latest_state(state)?;

        if latest < state.last_block {
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
    pub authoring_version: u64,
    #[serde(rename = "implName")]
    pub impl_name: String,
    #[serde(rename = "implVersion")]
    pub impl_version: u64,
    #[serde(rename = "specName")]
    pub spec_name: String,
    #[serde(rename = "specVersion")]
    pub spec_version: u64,
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
