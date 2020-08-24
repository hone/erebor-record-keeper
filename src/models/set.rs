use sqlx::postgres::PgPool;

pub struct Set {
    pub id: i64,
    pub name: String,
}

impl Set {
    pub async fn find_all(pool: &PgPool) -> anyhow::Result<Vec<Set>> {
        Ok(sqlx::query_as!(
            Set,
            r#"
SELECT id, name
FROM sets
"#,
        )
        .fetch_all(pool)
        .await?)
    }
}
