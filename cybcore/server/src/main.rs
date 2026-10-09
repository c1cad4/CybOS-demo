use axum::{extract::State, http::{HeaderMap, StatusCode}, routing::get, Json, Router};
use serde::Serialize;
use sqlx::PgPool;
use std::{env, net::SocketAddr};

#[derive(Clone)]
struct AppState {
    db: PgPool,
    status_token: String,
}

#[derive(Serialize)]
struct Health {
    service: &'static str,
    status: &'static str,
    version: &'static str,
}

async fn health() -> Json<Health> {
    Json(Health {
        service: "cybcore",
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

fn authorized(headers: &HeaderMap, token: &str) -> bool {
    let Some(value) = headers.get("authorization").and_then(|v| v.to_str().ok()) else {
        return false;
    };
    let Some(provided) = value.strip_prefix("Bearer ") else {
        return false;
    };
    // Do not leak secret length or short-circuit on matching prefix.
    if provided.len() != token.len() { return false; }
    let mut diff = 0u8;
    for (a, b) in provided.bytes().zip(token.bytes()) { diff |= a ^ b; }
    diff == 0
}

async fn ready(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Health>, StatusCode> {
    if !authorized(&headers, &state.status_token) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(Json(Health {
        service: "cybcore",
        status: "ready",
        version: env!("CARGO_PKG_VERSION"),
    }))
}

fn app(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(health))
        .route("/readyz", get(ready))
        .with_state(state)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = env::var("DATABASE_URL").map_err(|_| "DATABASE_URL must be set")?;
    let status_token = env::var("CYBCORE_STATUS_TOKEN")
        .map_err(|_| "CYBCORE_STATUS_TOKEN must be set")?;
    if status_token.len() < 32 || status_token.starts_with("REPLACE_") {
        return Err("CYBCORE_STATUS_TOKEN must be a non-placeholder secret of at least 32 bytes".into());
    }
    let db = PgPool::connect(&url).await?;
    sqlx::migrate!("./migrations").run(&db).await?;
    let bind: SocketAddr = env::var("CYBCORE_BIND")
        .unwrap_or_else(|_| "0.0.0.0:8080".into())
        .parse()?;
    let listener = tokio::net::TcpListener::bind(bind).await?;
    eprintln!("CybCore listening on {bind}");
    axum::serve(listener, app(AppState { db, status_token }))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn readiness_requires_bearer_token() {
        let mut headers = HeaderMap::new();
        assert!(!authorized(&headers, "a-valid-secret"));
        headers.insert("authorization", "Bearer wrong".parse().unwrap());
        assert!(!authorized(&headers, "a-valid-secret"));
        headers.insert("authorization", "Bearer a-valid-secret".parse().unwrap());
        assert!(authorized(&headers, "a-valid-secret"));
    }
    #[tokio::test]
    async fn health_response_is_explicit() {
        let Json(h) = health().await;
        assert_eq!(h.service, "cybcore");
        assert_eq!(h.status, "ok");
    }
}
