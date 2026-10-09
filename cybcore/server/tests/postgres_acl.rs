//! Run with TEST_DATABASE_URL pointing to a disposable PostgreSQL database.
//! Test database must not contain production data.
use sqlx::PgPool;

#[tokio::test]
async fn trusted_node_permissions_are_default_deny_and_revocable() {
    let Ok(url) = std::env::var("TEST_DATABASE_URL") else {
        eprintln!("Skipping PostgreSQL integration test: TEST_DATABASE_URL unset");
        return;
    };
    let pool = PgPool::connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let key = "a".repeat(64);
    let kind = "test.integration.heartbeat";
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO trusted_nodes(author_key,label) VALUES($1,$2) ON CONFLICT(author_key) DO UPDATE SET enabled=TRUE")
        .bind(&key).bind("integration test node").execute(&mut *tx).await.unwrap();
    let permitted: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM trusted_nodes n JOIN node_event_permissions p USING(author_key) WHERE n.author_key=$1 AND n.enabled AND p.kind=$2)"
    ).bind(&key).bind(kind).fetch_one(&mut *tx).await.unwrap();
    assert!(!permitted, "ungranted event kinds must be denied");
    sqlx::query("INSERT INTO node_event_permissions(author_key,kind) VALUES($1,$2) ON CONFLICT DO NOTHING")
        .bind(&key).bind(kind).execute(&mut *tx).await.unwrap();
    let permitted: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM trusted_nodes n JOIN node_event_permissions p USING(author_key) WHERE n.author_key=$1 AND n.enabled AND p.kind=$2)"
    ).bind(&key).bind(kind).fetch_one(&mut *tx).await.unwrap();
    assert!(permitted);
    sqlx::query("UPDATE trusted_nodes SET enabled=FALSE WHERE author_key=$1")
        .bind(&key).execute(&mut *tx).await.unwrap();
    let permitted: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM trusted_nodes n JOIN node_event_permissions p USING(author_key) WHERE n.author_key=$1 AND n.enabled AND p.kind=$2)"
    ).bind(&key).bind(kind).fetch_one(&mut *tx).await.unwrap();
    assert!(!permitted, "revoked node must be denied");
    let other_kind = "test.integration.denied";
    let denied: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM trusted_nodes n JOIN node_event_permissions p USING(author_key) WHERE n.author_key=$1 AND n.enabled AND p.kind=$2)"
    ).bind(&key).bind(other_kind).fetch_one(&mut *tx).await.unwrap();
    assert!(!denied, "grant for one event kind must not grant another");
    tx.rollback().await.unwrap();
}
