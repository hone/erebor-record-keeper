use sqlx::postgres::PgPool;

pub struct User {
    pub id: i64,
    pub discord_id: i64,
    pub name: Option<String>,
}

impl User {
    /// Find or create User
    pub async fn find_or_create(
        pool: &PgPool,
        discord_id: &u64,
        name: &str,
    ) -> anyhow::Result<User> {
        let discord_id_pg = *discord_id as i64;

        sqlx::query!(
            r#"
INSERT INTO users (discord_id, name)
VALUES ($1, $2)
ON CONFLICT (discord_id)
DO
    UPDATE SET name = $2,
        updated_at = CURRENT_TIMESTAMP
        "#,
            discord_id_pg,
            name
        )
        .execute(pool)
        .await?;

        sqlx::query_as!(
            crate::models::event::Event,
            r#"
SELECT id, name
FROM events
WHERE active = $1
"#,
            true
        )
        .fetch_all(pool)
        .await?;

        let user = sqlx::query_as!(
            User,
            r#"
SELECT id, discord_id, name
FROM users
WHERE discord_id = $1
"#,
            discord_id_pg
        )
        .fetch_one(pool)
        .await?;

        Ok(user)
    }
}
