use axum::{
    routing::get,
    http::StatusCode,
    Json,
    Router
};
use futures::join;
use parse_int::parse;
use serde::{Deserialize, Serialize};
use serde_json::{json,Value};
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
struct EthBlockNumberResponse {
    id: u8,
    result: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct BlockNumberByProvider {
    provider: String,
    block_number: Option<u32>,
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let app = Router::new()
        .route("/", get(compare_heads));

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn compare_heads() -> (StatusCode, Json<Value>) {
    let future_rpc = get_block_number_rpc();
    let future_rpc2 = get_block_number_rpc2();
    let future_infura = get_block_number_infura();
    let future_bordel = get_block_number_bordel();

    let (
        block_number_rpc,
        block_number_rpc2,
        block_number_infura,
        block_number_bordel
    ) = join!(
        future_rpc,
        future_rpc2,
        future_infura,
        future_bordel
    );

    let mut digest: Vec<BlockNumberByProvider> = Vec::<BlockNumberByProvider>::new();
    digest.push(block_number_rpc);
    digest.push(block_number_rpc2);
    digest.push(block_number_infura);
    digest.push(block_number_bordel);

    (StatusCode::OK, Json(json!(digest)))
}

async fn get_block_number_rpc() -> BlockNumberByProvider {
    let block_number = get_block_number("https://rpc.sepolia.org").await;
    block_number_by_provider("rpc", block_number)
}

async fn get_block_number_rpc2() -> BlockNumberByProvider {
    let block_number = get_block_number("https://rpc2.sepolia.org").await;
    block_number_by_provider("rpc2", block_number)
}

async fn get_block_number_infura() -> BlockNumberByProvider {
    let env_infura_key: Option<String> = dotenv::var("INFURA_KEY").ok();
    let url: &str = &*format!("https://sepolia.infura.io/v3/{}", env_infura_key.unwrap());
    let block_number = get_block_number(url).await;

    block_number_by_provider("infura", block_number)
}

async fn get_block_number_bordel() -> BlockNumberByProvider {
    let block_number = get_block_number("https://rpc.bordel.wtf/sepolia").await;
    block_number_by_provider("bordel", block_number)
}

fn block_number_by_provider(provider: &str, block_number: Option<u32>) -> BlockNumberByProvider {
    BlockNumberByProvider {
        provider: String::from(provider),
        block_number: block_number,
    }
}

async fn get_block_number(url: &str) -> Option<u32> {
    match fetch_block_number_response(url).await {
        Ok(response) => {
            let block_number = parse_block_number_response(response);
            log::info!("Fetched {} -> {:?}", url, block_number);

            block_number
        }
        Err(error) => {
            log::warn!("Could not fetch block number from: {}", url);
            log::warn!("Fetch error: {:?}", error);

            None
        }

    }
}

fn parse_block_number_response(response: EthBlockNumberResponse) -> Option<u32> {
    parse::<u32>(&response.result.to_string()).ok()
}

async fn fetch_block_number_response(url: &str) -> Result<EthBlockNumberResponse, reqwest::Error> {
    log::info!("Fetching: {}", url);

    let json_response: EthBlockNumberResponse = reqwest::Client::builder()
        .timeout(Duration::from_millis(3000))
        .build()?
        .post(url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_blockNumber",
            "params": [],
            "id": 0
        }))
        .send()
        .await?
        .json()
        .await?;

    Ok(json_response)
}
