use axum::{
    extract::State,
    routing::post,
    Json, Router,
};
use ed25519_dalek::{Signer, SigningKey};
use rand::Rng;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
struct AppState {
    db: Arc<Mutex<Connection>>,
    signing_key: SigningKey,
}

#[derive(Deserialize)]
struct ActivateRequest {
    device_id: String,
    product_key: String,
}

#[derive(Serialize)]
struct TokenResponse {
    token: String,
    signature: String,
}

#[derive(Serialize)]
struct TokenPayload {
    device_id: String,
    valid_until: u64,
    tier: String,
}

fn hex_encode(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02x}", b)).collect()
}

async fn activate(
    State(state): State<AppState>,
    Json(payload): Json<ActivateRequest>,
) -> Json<TokenResponse> {
    let db = state.db.lock().unwrap();
    db.execute(
        "INSERT OR IGNORE INTO activations (device_id, product_key) VALUES (?1, ?2)",
        [&payload.device_id, &payload.product_key],
    ).ok();

    let valid_until = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() + (14 * 24 * 60 * 60);

    let token_payload = TokenPayload {
        device_id: payload.device_id,
        valid_until,
        tier: "premium".to_string(),
    };

    let token_json = serde_json::to_string(&token_payload).unwrap();
    let signature = state.signing_key.sign(token_json.as_bytes());

    Json(TokenResponse {
        token: token_json,
        signature: hex_encode(&signature.to_bytes()),
    })
}

async fn lease(
    State(state): State<AppState>,
    Json(payload): Json<ActivateRequest>,
) -> Json<TokenResponse> {
    // Equivalent for the testbed
    activate(State(state), Json(payload)).await
}

#[tokio::main]
async fn main() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute(
        "CREATE TABLE IF NOT EXISTS activations (
            id INTEGER PRIMARY KEY,
            device_id TEXT NOT NULL,
            product_key TEXT NOT NULL
        )",
        [],
    ).unwrap();

    let sk_bytes: [u8; 32] = rand::random();
    let signing_key = SigningKey::from_bytes(&sk_bytes);

    let state = AppState {
        db: Arc::new(Mutex::new(conn)),
        signing_key,
    };

    let app = Router::new()
        .route("/activate", post(activate))
        .route("/lease", post(lease))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("License server listening on port 3000");
    axum::serve(listener, app).await.unwrap();
}
