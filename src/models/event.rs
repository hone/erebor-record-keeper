use sqlx::{postgres::PgPool, prelude::Done};

pub struct Event {
    pub id: i64,
    pub name: String,
}

impl Event {
    /// Find the active Event
    pub async fn find_by_active(pool: &PgPool, active: bool) -> anyhow::Result<Option<Event>> {
        let mut rows = sqlx::query_as!(
            Event,
            r#"
SELECT id, name
FROM events
WHERE active = $1
"#,
            active
        )
        .fetch_all(pool)
        .await?;

        Ok(rows.pop())
    }

    /// Find events by archive status
    pub async fn find_by_archive(pool: &PgPool, archive: bool) -> anyhow::Result<Vec<Event>> {
        Ok(sqlx::query_as!(
            Event,
            r#"
SELECT id, name
FROM events
WHERE archive = $1
"#,
            archive
        )
        .fetch_all(pool)
        .await?)
    }

    /// Create new event
    pub async fn create(pool: &PgPool, name: &str) -> anyhow::Result<u64> {
        Ok(
            sqlx::query!(r#"INSERT INTO events ( name ) VALUES ( $1 )"#, name)
                .execute(pool)
                .await?
                .rows_affected(),
        )
    }
}
