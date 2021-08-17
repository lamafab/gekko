use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::time::{sleep, Duration};

const BLOCK_HASH_LIMIT: u64 = 50;
const TIMEOUT: u64 = 10;

type Result<T> = std::result::Result<T, anyhow::Error>;

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

        let header = get::<Vec<u64>, Vec<String>>(RpcMethod::BlockHash, range).await?;
        let version = get::<(), RuntimeVersion>(RpcMethod::RuntimeVersion, ()).await?;

        current_block = to + 1;

        if latest < current_block {
            sleep(Duration::from_secs(TIMEOUT)).await;
            latest = latest_block().await?;
        }
    }

    Ok(())
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
