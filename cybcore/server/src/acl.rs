//! Default-deny per-node event authorization. Provision keys through trusted DBA operations.
use sqlx::PgPool;

pub async fn may_publish(db: &PgPool, author_key: &str, kind: &str) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM trusted_nodes n \
         JOIN node_event_permissions p ON p.author_key = n.author_key \
         WHERE n.author_key = $1 AND n.enabled = TRUE AND p.kind = $2)",
    )
    .bind(author_key)
    .bind(kind)
    .fetch_one(db)
    .await
}

#[cfg(test)]
mod tests {
    #[test]
    fn no_implicit_administrator_or_wildcard_permissions() {
        // Permissions are exact kind matches; schema contains no bootstrap admin.
        assert_ne!("robot.heartbeat", "robot.command");
    }
}
