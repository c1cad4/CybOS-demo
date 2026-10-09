mod events;
mod acl;
use axum::{extract::State, http::{HeaderMap, StatusCode}, routing::{get, post}, Json, Router};
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

async fn ingest_event(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(event): Json<events::SignedEvent>,
) -> Result<StatusCode, StatusCode> {
    // Internal-only ingestion until per-node ACL and tenant scopes exist.
    if !authorized(&headers, &state.status_token) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .as_millis();
    let now_ms = i64::try_from(now_ms).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    event.verify(now_ms).map_err(|_| StatusCode::BAD_REQUEST)?;
    if !acl::may_publish(&state.db, &event.author_key, &event.kind)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)? {
        return Err(StatusCode::FORBIDDEN);
    }
    let inserted = sqlx::query(
        "INSERT INTO signed_events (author_key, event_id, kind, created_at_ms, content, signature) \
         VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT DO NOTHING",
    )
    .bind(&event.author_key)
    .bind(&event.event_id)
    .bind(&event.kind)
    .bind(event.created_at_ms)
    .bind(&event.content)
    .bind(&event.signature)
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    if inserted.rows_affected() == 0 {
        return Err(StatusCode::CONFLICT);
    }
    Ok(StatusCode::CREATED)
}

fn app(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(health))
        .route("/readyz", get(ready))
        .route("/v1/events", post(ingest_event))
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
