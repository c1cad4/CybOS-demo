use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use serde::Serialize;
use sqlx::PgPool;
use std::{env, net::SocketAddr};

#[derive(Clone)]
struct AppState {
    db: PgPool,
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

async fn ready(State(state): State<AppState>) -> Result<Json<Health>, StatusCode> {
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
    let db = PgPool::connect(&url).await?;
    sqlx::migrate!("./migrations").run(&db).await?;
    let bind: SocketAddr = env::var("CYBCORE_BIND")
        .unwrap_or_else(|_| "0.0.0.0:8080".into())
        .parse()?;
    let listener = tokio::net::TcpListener::bind(bind).await?;
    eprintln!("CybCore listening on {bind}");
    axum::serve(listener, app(AppState { db }))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn health_response_is_explicit() {
        let Json(h) = health().await;
        assert_eq!(h.service, "cybcore");
        assert_eq!(h.status, "ok");
    }
}
