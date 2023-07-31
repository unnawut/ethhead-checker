use axum::{
    routing::get,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
    Router
};
use parse_int::parse;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Serialize, Deserialize)]
struct EthBlockNumberResponse {
    id: u8,
    result: String,
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

async fn compare_heads() -> impl IntoResponse {
    let env_infura_key: Option<String> = dotenv::var("INFURA_KEY").ok();
    let url: &str = &*format!("https://sepolia.infura.io/v3/{}", env_infura_key.unwrap());

    match get_block_number(url).await {
        Some(block_number) => {
            log::info!("Found infura block number: {}", block_number);
            (StatusCode::OK, Json(json!({ "infura": block_number })))
        }
        None => {
            log::error!("Could not look up block number from Infura: {}", url);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error_message": "Could not look up block number from Infura" })))
        }
    }

}

async fn get_block_number(url: &str) -> Option<u32> {
    match fetch_block_number_response(url).await {
        Ok(response) => {
            match parse::<u32>(&response.result.to_string()) {
                Ok(parsed_block_number) => Some(parsed_block_number),
                _ => None
            }
        }
        _ => None
    }
}

async fn fetch_block_number_response(url: &str) -> Result<EthBlockNumberResponse, reqwest::Error> {
    log::debug!("Querying: {}", url);

    let json_response: EthBlockNumberResponse = reqwest::Client::new()
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
}

impl From<reqwest::Error> for AppError {
    fn from(error: reqwest::Error) -> Self {
        AppError::FetchError(error)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::FetchError(error) => {
                log::error!("Could not fetch block number from: {:?}", error.url());
                (StatusCode::INTERNAL_SERVER_ERROR, "Could not fetch block number" )
            }
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}
