use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

type Result<T> = std::result::Result<T, anyhow::Error>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest<'a, T> {
    pub id: i64,
    pub jsonrpc: &'a str,
    pub method: &'a str,
    pub params: Vec<T>,
}

impl<'a, T> RpcRequest<'a, T> {
    fn new(method: RpcMethod, params: Vec<T>) -> Self {
        RpcRequest {
            id: 1,
            jsonrpc: "2.0",
            method: method.as_str(),
            params: params,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
    pub jsonrpc: String,
    pub result: Vec<String>,
    pub id: i64,
}

#[derive(Debug, Clone, Copy)]
enum RpcMethod {
    Block,
    BlockHash,
    RuntimeVersion,
    Metadata,
}

impl RpcMethod {
    fn as_str(&self) -> &'static str {
        match self {
            Block => "chain_getBlock",
            BlockHash => "chain_getBlockHash",
            RuntimeVersion => "state_getRuntimeVersion",
            Metadata => "state_getMetadata",
        }
    }
}

pub fn run() {}

async fn get<B: Serialize, R: DeserializeOwned>(method: RpcMethod, body: Vec<B>) -> Result<R> {
    Client::new()
        .post(method.as_str())
        .json(&RpcRequest::new(method, body))
        .send()
        .await?
        .json::<R>()
        .await
        .map_err(|err| err.into())
}
