use crate::utils::PostgresPool;
use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::channel::Message,
    prelude::Context,
};

#[command]
#[aliases("pain")]
#[usage = ""]
#[example = ""]
/// Display a random gautlet challenge
pub async fn gauntlet(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    let pool = data
        .get::<PostgresPool>()
        .expect("Expected PostgresPool in TypeMap.");

    let result = sqlx::query!(
        r#"
SELECT name, description
FROM challenges 
WHERE 'Gauntlet' = ANY(attributes)
ORDER BY RANDOM()
LIMIT 1
"#,
    )
    .fetch_all(pool)
    .await?
    .pop();

    let message = if let Some(challenge) = result {
        format!(
            "You've selected the following challenge:\n*{}* - {}",
            challenge.name,
            challenge.description.unwrap_or("".to_string())
        )
    } else {
        "Could not find any challenges.".to_string()
    };

    msg.reply(&ctx.http, message).await?;

    Ok(())
}
