#[macro_use]
extern crate log;

use anyhow::anyhow;
use log::LevelFilter;
use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fs::{read_to_string, File};
use std::io::prelude::*;
use std::path::Path;
use tokio::sync::mpsc::{channel, Sender};
use tokio::time::{sleep, Duration};

const BLOCK_HASH_LIMIT: u64 = 30;
const TIMEOUT: u64 = 10;

pub type Result<T> = std::result::Result<T, anyhow::Error>;

// TODO
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    collectors: Vec<CollectorConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CollectorConfig {
    chain_name: String,
    endpoint: String,
    directory: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RpcRequest<'a, T> {
    id: i64,
    jsonrpc: &'a str,
    method: &'a str,
    params: T,
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
struct RpcResponse<T> {
    jsonrpc: String,
    result: T,
    id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    pub jsonrpc: String,
    pub error: Error,
    pub id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Error {
    pub code: i64,
    pub message: String,
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

fn read_config() -> Result<Config> {
    serde_yaml::from_str(&read_to_string("config.yaml")?).map_err(|err| err.into())
}

pub async fn run() -> Result<()> {
    env_logger::Builder::new()
        .filter_module("system", LevelFilter::Debug)
        .init();

    info!("Reading config...");
    let config = read_config()?;
    let (tx, mut recv) = channel::<()>(1);

    for collector in config.collectors {
        info!("Starting collector for {}", collector.chain_name);
        tokio::spawn(do_run(tx.clone(), collector));
    }

    // Wait for shutdown signal.
    recv.recv().await;

    Err(anyhow!("service is shutting down unexpectedly"))
}

async fn do_run(tx: Sender<()>, config: CollectorConfig) {
    async fn local(config: CollectorConfig) -> Result<()> {
        let fs = Filesystem::new(config.clone());
        let url = config.endpoint;

        // Fetch the latest known block number.
        info!("Fetching latest block number");
        let mut latest = latest_block(&url).await?;

        // Retrieve the last block number from where data should start being fetched from.
        info!("Reading state from disk");
        let mut state = fs.read_last_state()?.unwrap_or(LatestInfo {
            spec_version: 0,
            last_block: 0,
        });

        info!("Starting event loop");
        loop {
            // Set range of block numbers, do not exceed limit.
            let from = state.last_block;
            let to = latest.min(state.last_block + BLOCK_HASH_LIMIT);
            let range = (from..=to).collect();

            debug!("Requesting hashes of blocks from number {} to {}", from, to);
            let header_hashes =
                get::<Vec<Vec<u64>>, Vec<String>>(&url, RpcMethod::BlockHash, vec![range]).await?;

            for hash in header_hashes {
                trace!("Fetching runtime version from state {}", hash);
                let version =
                    get::<_, RuntimeVersion>(&url, RpcMethod::RuntimeVersion, vec![hash.clone()])
                        .await?;

                trace!("Fetching metadata from state {}", hash);
                let metadata = get::<_, MetadataHex>(&url, RpcMethod::Metadata, vec![hash]).await?;

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
                    trace!(
                        "No new version found at block {}, continuing",
                        version.spec_version
                    );
                }

                state.spec_version = version.spec_version;
                fs.track_latest_state(&state)?;
                state.last_block += 1;
            }

            if latest < state.last_block {
                sleep(Duration::from_secs(TIMEOUT)).await;
                latest = latest_block(&url).await?;
            }
        }
    }

    let name = config.chain_name.clone();
    if let Err(err) = local(config).await {
        error!("Error in {} collector: {:?}", name, err);
    }

    // Send shutdown signal.
    let _ = tx.send(()).await.unwrap();
}

/// Convenience function for fetching the latest block number.
async fn latest_block(url: &str) -> Result<u64> {
    get::<Option<()>, Header>(url, RpcMethod::Header, None)
        .await
        .and_then(|header| u64::from_str_radix(&header.number[2..], 16).map_err(|err| err.into()))
        .map_err(|err| anyhow::Error::from(err))
}

/// Response when calling `chain_getHeader`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Header {
    digest: serde_json::Value,
    #[serde(rename = "extrinsicsRoot")]
    extrinsics_root: String,
    number: String,
    #[serde(rename = "parentHash")]
    parent_hash: String,
    #[serde(rename = "stateRoot")]
    state_root: String,
}

/// Response when calling `state_getRuntimeVersion`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RuntimeVersion {
    apis: Vec<(String, i64)>,
    #[serde(rename = "authoringVersion")]
    authoring_version: u64,
    #[serde(rename = "implName")]
    impl_name: String,
    #[serde(rename = "implVersion")]
    impl_version: u64,
    #[serde(rename = "specName")]
    spec_name: String,
    #[serde(rename = "specVersion")]
    spec_version: u64,
}

/// Response when calling `state_getMetadata`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MetadataHex(String);

/// Convenience function for executing a RPC call.
async fn get<B: Serialize, R: std::fmt::Debug + DeserializeOwned>(
    url: &str,
    method: RpcMethod,
    body: B,
) -> Result<R> {
    Client::new()
        .post(url)
        .json(&RpcRequest::new(method, body))
        .send()
        .await?
        .text()
        .await
        .map_err(|err| err.into())
        .and_then(|data| {
            let res = serde_json::from_str::<RpcResponse<R>>(&data);

            if let Ok(ok) = res {
                Ok(ok.result)
            } else if let Ok(err) = serde_json::from_str::<RpcError>(&data) {
                Err(anyhow!(
                    "received error message from connected node: {:?}",
                    err
                ))
            } else {
                Err(res.unwrap_err().into())
            }
        })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LatestInfo {
    spec_version: u64,
    last_block: u64,
}

/// Handler to save the collected information to disk.
struct Filesystem {
    config: CollectorConfig,
}

impl Filesystem {
    const STATE: &'static str = ".collection_state";

    fn new(config: CollectorConfig) -> Self {
        Filesystem { config: config }
    }
    fn path(&self) -> String {
        format!(
            "{}/",
            self.config
                .directory
                .as_ref()
                .map(|dir| dir.as_str())
                // Current 'pwd'.
                .unwrap_or(""),
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
