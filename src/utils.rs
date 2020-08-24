use crate::models::set::Set;
use serenity::{model::channel::Message, prelude::Context, prelude::TypeMapKey};
use std::time::Duration;

pub const SELECTION_TIMEOUT: u64 = 60;

pub struct PostgresPool;
impl TypeMapKey for PostgresPool {
    type Value = sqlx::postgres::PgPool;
}

/// Format collection into a 1-indexed joint String
pub fn format_collection<T: std::fmt::Display>(collection: &Vec<T>) -> String {
    let width = collection.len() / 10;
    collection
        .iter()
        .enumerate()
        .map(|(i, item)| format!("{:>width$}.) {}", i + 1, item, width = width))
        .collect::<Vec<String>>()
        .join("\n")
}

/// Ask to pick from a collection and return the id of the set
pub async fn pick_collection<'a, T>(
    ctx: &Context,
    msg: &Message,
    collection: &'a Vec<T>,
) -> anyhow::Result<Option<&'a T>> {
    if let Some(choice) = &msg
        .author
        .await_reply(&ctx)
        .timeout(Duration::from_secs(SELECTION_TIMEOUT))
        .await
    {
        let number = choice.content.parse::<usize>()?;
        let index = number - 1;

        Ok(collection.get(index))
    } else {
        Ok(None)
    }
}

/// Ask to pick from all sets and return the set id picked
pub async fn pick_sets(ctx: &Context, msg: &Message) -> anyhow::Result<Option<i64>> {
    let data = ctx.data.read().await;
    let pool = data
        .get::<PostgresPool>()
        .expect("Expected PostgresPool in TypeMap.");
    let sets = Set::find_all(pool).await?;
    msg.channel_id
        .say(
            &ctx.http,
            format!(
                "Here are all the sets:\n{}",
                format_collection(&sets.iter().map(|set| &set.name).collect())
            ),
        )
        .await?;

    let set_id = pick_collection(ctx, msg, &sets).await?.map(|set| set.id);
    Ok(set_id)
}
