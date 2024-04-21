use axum::{
  response::{Html, IntoResponse, Response},
  routing::{get, post},
  Json, Router,
};
use bitcoin::{address::NetworkUnchecked, Address, Amount};
use http::StatusCode;
use log::debug;
use ord::FeeRate;
use ordinals::SpacedRune;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
  env_logger::init();
  // build our application with a route
  let app = Router::new()
    .route("/", get(handler))
    .route("/mint", post(mint_handler));

  // run it
  let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
  println!("listening on {}", addr);
  axum::Server::bind(&addr)
    .serve(app.into_make_service())
    .await
    .unwrap();
}

async fn handler() -> Html<&'static str> {
  Html("<h1>Hello, World!</h1>")
}

#[derive(Debug, Deserialize)]
struct MintParams {
  fee_rate: FeeRate,
  rune: SpacedRune,
  postage: Option<BtcAmount>,
  destination: Option<Address<NetworkUnchecked>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(transparent)]
struct BtcAmount(#[serde(with = "bitcoin::amount::serde::as_btc")] Amount);

#[axum::debug_handler]
async fn mint_handler(Json(params): Json<MintParams>) -> Result<Vec<u8>, AppError> {
  let MintParams {
    fee_rate,
    rune,
    postage,
    destination,
  } = params;
  use ord::subcommand::wallet::mint::{RunesMint, WalletParams};
  let runes_mint = RunesMint {
    fee_rate,
    rune,
    postage: postage.map(|postage| postage.0),
    destination,
  };

  debug!("{runes_mint:?}");

  let res = runes_mint.run_in_place(WalletParams {
    name: "test".into(),
    no_sync: false,
    server_url: None,
  })?;

  Ok(res)
}

// Make our own error that wraps `anyhow::Error`.
struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
  fn into_response(self) -> Response {
    (
      StatusCode::INTERNAL_SERVER_ERROR,
      format!("Something went wrong: {}", self.0),
    )
      .into_response()
  }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
where
  E: Into<anyhow::Error>,
{
  fn from(err: E) -> Self {
    Self(err.into())
  }
}
