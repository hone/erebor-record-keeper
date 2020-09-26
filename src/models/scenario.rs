use sqlx::postgres::PgPool;

pub struct Scenario {
    pub id: i64,
    pub title: String,
    pub code: String,
    pub set_id: i64,
    pub number: Option<i16>,
}

impl Scenario {
    pub async fn find_by_title(pool: &PgPool, title: &str) -> anyhow::Result<Option<Scenario>> {
        let mut scenarios = sqlx::query_as!(
            Scenario,
            r#"
SELECT id, title, code, set_id, number
FROM scenarios
WHERE title = $1
"#,
            title
        )
        .fetch_all(pool)
        .await?;

        Ok(scenarios.pop())
    }
}
