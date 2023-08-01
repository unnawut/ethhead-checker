use axum::{
    routing::get,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
    Router
};
use core::num::ParseIntError;
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
    block_number: u32,
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

async fn get_block_number_infura() -> Result<BlockNumberByProvider, AppError> {
    let env_infura_key: Option<String> = dotenv::var("INFURA_KEY").ok();
    let url: &str = &*format!("https://sepolia.infura.io/v3/{}", env_infura_key.unwrap());

    log::info!("Fetching: {}", url);
    let block_number = get_block_number(url).await?;
    log::info!("Found infura block number: {}", block_number);

    Ok(BlockNumberByProvider {
        provider: String::from("infura"),
        block_number: block_number,
    })
}

async fn get_block_number_bordel() -> Result<BlockNumberByProvider, AppError> {
    let url = "https://rpc.bordel.wtf/sepolia";

    log::info!("Fetching: {}", url);
    let block_number = get_block_number(url).await?;
    log::info!("Found bordel block number: {}", block_number);

    Ok(BlockNumberByProvider {
        provider: String::from("bordel"),
        block_number: block_number,
    })
}

async fn compare_heads() -> Result<(StatusCode, Json<Value>), AppError> {
    let future_infura = get_block_number_infura();
    let future_bordel = get_block_number_bordel();

    let (block_number_infura, block_number_bordel) = join!(future_infura, future_bordel);

    let mut digest: Vec<BlockNumberByProvider> = Vec::<BlockNumberByProvider>::new();
    digest.push(block_number_infura?);
    digest.push(block_number_bordel?);

    Ok((StatusCode::OK, Json(json!(digest))))
}

async fn get_block_number(url: &str) -> Result<u32, AppError> {
    let response = fetch_block_number_response(url).await?;
    Ok(parse::<u32>(&response.result.to_string())?)
}

async fn fetch_block_number_response(url: &str) -> Result<EthBlockNumberResponse, AppError> {
    log::debug!("Querying: {}", url);

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

    log::debug!("Fetched: {:?}", json_response);
    Ok(json_response)
}

enum AppError {
    FetchError(reqwest::Error),
    ParseError(ParseIntError),
}

impl From<reqwest::Error> for AppError {
    fn from(error: reqwest::Error) -> Self {
        AppError::FetchError(error)
    }
}

impl From<ParseIntError> for AppError {
    fn from(error: ParseIntError) -> Self {
        AppError::ParseError(error)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::FetchError(error) => {
                log::error!("Could not fetch block number from: {:?}", error.url());
                (StatusCode::INTERNAL_SERVER_ERROR, "Could not fetch block number" )
            }
            AppError::ParseError(error) => {
                log::error!("Could not parse block number: {:?}", error);
                (StatusCode::INTERNAL_SERVER_ERROR, "Could not parse block number" )
            }
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}
