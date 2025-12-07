use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use crate::state::AppState;
use bcrypt::{hash, verify, DEFAULT_COST};
use jsonwebtoken::{encode, EncodingKey, Header};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct AuthPayload {
    username: String,
    password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    token: String,
    username: String,
}

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String, // username
    exp: usize,
}

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<AuthPayload>,
) -> impl IntoResponse {
    // Check if user exists
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM accounts WHERE username = ?)")
        .bind(&payload.username)
        .fetch_one(&state.db.pool)
        .await
        .unwrap_or(false);

    if exists {
        return (StatusCode::CONFLICT, "Username already exists").into_response();
    }

    // Hash password
    let hashed = hash(payload.password, DEFAULT_COST).unwrap();
    let id = Uuid::new_v4().to_string();

    // Save to DB
    let result = sqlx::query("INSERT INTO accounts (id, username, password_hash) VALUES (?, ?, ?)")
        .bind(id)
        .bind(&payload.username)
        .bind(hashed)
        .execute(&state.db.pool)
        .await;

    match result {
        Ok(_) => (StatusCode::CREATED, "Account created").into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create account").into_response(),
    }
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<AuthPayload>,
) -> impl IntoResponse {
    let row: Option<(String, String)> = sqlx::query_as("SELECT username, password_hash FROM accounts WHERE username = ?")
        .bind(&payload.username)
        .fetch_optional(&state.db.pool)
        .await
        .unwrap_or(None);

    if let Some((username, hash)) = row {
        if verify(payload.password, &hash).unwrap_or(false) {
            // Generate JWT
            let expiration = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as usize + 3600 * 24; // 24 hours

            let claims = Claims {
                sub: username.clone(),
                exp: expiration,
            };

            let token = encode(&Header::default(), &claims, &EncodingKey::from_secret("secret".as_ref())).unwrap();

            return Json(AuthResponse { token, username }).into_response();
        }
    }

    (StatusCode::UNAUTHORIZED, "Invalid credentials").into_response()
}
