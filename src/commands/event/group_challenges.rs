//! Collection of commands for Events with challeneges with group wide completion
use crate::{
    models::{challenge::Challenge, event::Event},
    utils::{self, PostgresPool},
};
use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::channel::Message,
    prelude::Context,
};

#[command]
#[usage = ""]
#[example = ""]
/// List all group challenges
pub async fn cgroupall(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    let pool = data
        .get::<PostgresPool>()
        .expect("Expected PostgresPool in TypeMap.");

    let event = match Event::find_by_active(pool, true).await? {
        Some(event) => event,
        None => {
            msg.channel_id
                .say(&ctx.http, "No active event found.")
                .await?;

            return Ok(());
        }
    };

    let challenges = Challenge::find_all_by_event(&pool, event.id).await?;

    if challenges.is_empty() {
        msg.channel_id
            .say(&ctx.http, "No challenges found.")
            .await?;

        return Ok(());
    }

    let scenarios = crate::models::challenge::group_by_scenario(challenges);

    for content in utils::format_challenges_by_scenario(scenarios.iter()) {
        msg.channel_id.say(&ctx.http, content).await?;
    }

    Ok(())
}

#[command]
#[usage = ""]
#[example = ""]
/// List group challenges left
pub async fn cgroup(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    let pool = data
        .get::<PostgresPool>()
        .expect("Expected PostgresPool in TypeMap.");

    let event = match Event::find_by_active(pool, true).await? {
        Some(event) => event,
        None => {
            msg.channel_id
                .say(&ctx.http, "No active event found.")
                .await?;

            return Ok(());
        }
    };

    let challenges = Challenge::find_incompleted_by_event(&pool, event.id).await?;

    if challenges.is_empty() {
        msg.channel_id
            .say(&ctx.http, "No challenges found.")
            .await?;

        return Ok(());
    }

    let scenarios = crate::models::challenge::group_by_scenario(challenges);

    for content in utils::format_challenges_by_scenario(scenarios.iter()) {
        msg.channel_id.say(&ctx.http, content).await?;
    }

    Ok(())
}
