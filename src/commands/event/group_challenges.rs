//! Collection of commands for Events with challeneges with group wide completion
use crate::{
    models::{challenge, event::Event},
    utils::{self, PostgresPool},
};
use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::channel::Message,
    prelude::Context,
};

#[command]
#[aliases("challenges")]
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

    let challenges = event.find_incomplete_active_challenges(&pool).await?;

    if challenges.is_empty() {
        msg.channel_id
            .say(&ctx.http, "No challenges found.")
            .await?;

        return Ok(());
    }

    let scenarios = challenge::group_by_scenario(challenges);

    for content in utils::format_challenges_by_scenario(scenarios.iter()) {
        msg.channel_id.say(&ctx.http, content).await?;
    }

    Ok(())
}

#[command]
#[aliases("all-challenges")]
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

    let challenges = event.find_all_active_challenges(&pool).await?;

    if challenges.is_empty() {
        msg.channel_id
            .say(&ctx.http, "No challenges found.")
            .await?;

        return Ok(());
    }

    let scenarios = challenge::group_by_scenario(challenges);

    for content in utils::format_challenges_by_scenario(scenarios.iter()) {
        msg.channel_id.say(&ctx.http, content).await?;
    }

    Ok(())
}

#[command]
#[aliases("progress")]
#[usage = ""]
#[example = ""]
/// Show progress on group challenges
pub async fn cgroupprogress(ctx: &Context, msg: &Message) -> CommandResult {
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

    let all_challenges = event.find_all_challenges(&pool).await?;
    let completed_challenges = event.find_completed_challenges(&pool).await?;

    msg.channel_id
        .say(
            &ctx.http,
            format!(
                "We've completed {} of {} total challenges: {:.2}%",
                completed_challenges.len(),
                all_challenges.len(),
                (completed_challenges.len() as f32 / all_challenges.len() as f32) * 100.0
            ),
        )
        .await?;

    Ok(())
}
