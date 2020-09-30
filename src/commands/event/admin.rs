use crate::{models::event::Event, utils, utils::PostgresPool};
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::channel::Message,
    prelude::Context,
};
use sqlx::prelude::Done;
use std::time::Duration;

#[command]
/// Add scenarios to an event
pub async fn add(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    let pool = data
        .get::<PostgresPool>()
        .expect("Expected PostgresPool in TypeMap.");

    let events = Event::find_by_archive(pool, false).await?;
    if events.is_empty() {
        msg.channel_id
            .say(
                &ctx.http,
                "There are no unarchived events. Please create one.",
            )
            .await?;

        return Ok(());
    }

    msg.channel_id
        .say(
            &ctx.http,
            utils::format_collection(&events.iter().map(|event| &event.name).collect()),
        )
        .await?;
    let event;
    if let Some(e) = utils::pick_collection(ctx, msg, &events).await? {
        event = e;
    } else {
        return Ok(());
    }

    msg.channel_id
        .say(
            &ctx.http,
            r#"Do you want to add individual scenarios or by set?
1.) Scenarios
2.) Set
3.) All
"#,
        )
        .await?;

    if let Some(choice) = &msg
        .author
        .await_reply(&ctx)
        .timeout(Duration::from_secs(utils::SELECTION_TIMEOUT))
        .await
    {
        if choice.content == "1" {
            if let Some(set_id) = utils::pick_sets(ctx, msg).await? {
                let scenarios = sqlx::query!(
                    r#"
SELECT id, title
FROM scenarios
WHERE scenarios.set_id = $1
"#,
                    set_id
                )
                .fetch_all(pool)
                .await?;

                msg.channel_id
                    .say(
                        &ctx.http,
                        utils::format_collection(
                            &scenarios.iter().map(|scenario| &scenario.title).collect(),
                        ),
                    )
                    .await?;

                if let Some(scenario) = utils::pick_collection(ctx, msg, &scenarios).await? {
                    let row_count = sqlx::query!(
                        r#"
INSERT INTO events_scenarios ( event_id, scenario_id )
VALUES ( $1, $2 )
"#,
                        event.id,
                        scenario.id
                    )
                    .execute(pool)
                    .await?
                    .rows_affected();

                    msg.channel_id
                        .say(&ctx.http, format!("{} scenarios added.", row_count))
                        .await?;
                } else {
                    msg.channel_id.say(&ctx.http, "Not a valid choice.").await?;
                }
            } else {
                msg.channel_id.say(&ctx.http, "Not a valid index.").await?;
            }
        } else if choice.content == "2" {
            if let Some(set_id) = utils::pick_sets(ctx, msg).await? {
                let row_count = sqlx::query!(
                    r#"
INSERT INTO events_scenarios ( event_id, scenario_id )
SELECT $1, scenarios.id
FROM scenarios
WHERE scenarios.set_id = $2
    AND scenarios.id NOT IN (
        SELECT scenario_id
        FROM events_scenarios
        WHERE event_id = $1
  )
"#,
                    event.id,
                    set_id
                )
                .execute(pool)
                .await?
                .rows_affected();

                msg.channel_id
                    .say(&ctx.http, format!("{} scenarios added.", row_count))
                    .await?;
            } else {
                msg.channel_id.say(&ctx.http, "Not a valid index.").await?;
            }
        } else if choice.content == "3" {
            let row_count = sqlx::query!(
                r#"
INSERT INTO events_scenarios ( event_id, scenario_id )
SELECT $1, scenarios.id
FROM scenarios
WHERE scenarios.id NOT IN (
    SELECT scenario_id
    FROM events_scenarios
    WHERE event_id = $1
)
"#,
                event.id,
            )
            .execute(pool)
            .await?
            .rows_affected();

            msg.channel_id
                .say(&ctx.http, format!("{} scenarios added.", row_count))
                .await?;
        }
    };

    Ok(())
}

#[command]
/// Set event as active
pub async fn set(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    let pool = data
        .get::<PostgresPool>()
        .expect("Expected PostgresPool in TypeMap.");

    let events = sqlx::query!(
        r#"
SELECT id, name
FROM events
WHERE active = false
    AND archive = false
"#
    )
    .fetch_all(pool)
    .await?;

    msg.channel_id
        .say(
            &ctx.http,
            utils::format_collection(&events.iter().map(|event| &event.name).collect()),
        )
        .await?;

    if let Some(event) = utils::pick_collection(ctx, msg, &events).await? {
        sqlx::query!(
            r#"
UPDATE events
SET active = false,
    updated_at = CURRENT_TIMESTAMP
WHERE active = true
"#
        )
        .execute(pool)
        .await?;

        sqlx::query!(
            r#"
UPDATE events
SET active = true,
    updated_at = CURRENT_TIMESTAMP
WHERE id = $1"#,
            event.id
        )
        .execute(pool)
        .await?;

        msg.channel_id
            .say(
                &ctx.http,
                format!("'{}' is now the active event.", event.name),
            )
            .await?;
    }

    Ok(())
}

#[command]
/// Archive an event
pub async fn archive(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    let pool = data
        .get::<PostgresPool>()
        .expect("Expected PostgresPool in TypeMap.");

    let events = Event::find_by_archive(pool, false).await?;
    if events.is_empty() {
        msg.channel_id
            .say(
                &ctx.http,
                "There are no unarchived events. Please create one.",
            )
            .await?;

        return Ok(());
    }

    msg.channel_id
        .say(
            &ctx.http,
            utils::format_collection(&events.iter().map(|event| &event.name).collect()),
        )
        .await?;

    if let Some(event) = utils::pick_collection(ctx, msg, &events).await? {
        // make any active event inactive when archiving
        sqlx::query!(
            r#"
UPDATE events
SET active = false,
    archive = true,
    updated_at = CURRENT_TIMESTAMP
WHERE id = $1
"#,
            event.id
        )
        .execute(pool)
        .await?;

        msg.channel_id
            .say(&ctx.http, format!("'{}' is now archived.", event.name))
            .await?;
    } else {
        msg.channel_id
            .say(&ctx.http, "Could not archive event.")
            .await?;
    }

    Ok(())
}

#[command]
/// Load challenges for scenarios already registered for the event
pub async fn cload(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    let pool = data
        .get::<PostgresPool>()
        .expect("Expected PostgresPool in TypeMap.");

    let events = Event::find_by_archive(pool, false).await?;
    if events.is_empty() {
        msg.channel_id
            .say(
                &ctx.http,
                "There are no unarchived events. Please create one.",
            )
            .await?;

        return Ok(());
    }
    msg.channel_id
        .say(
            &ctx.http,
            utils::format_collection(&events.iter().map(|event| &event.name).collect()),
        )
        .await?;
    let event = match utils::pick_collection(ctx, msg, &events).await? {
        Some(event) => event,
        None => return Ok(()),
    };

    let rows_count = sqlx::query!(
        r#"
INSERT INTO challenges_events ( event_id, challenge_id )
SELECT $1, challenges.id
FROM events_scenarios, challenges
WHERE events_scenarios.event_id = $1
    AND challenges.scenario_id = events_scenarios.scenario_id
ON CONFLICT DO NOTHING
"#,
        event.id
    )
    .execute(pool)
    .await?
    .rows_affected();

    msg.channel_id
        .say(&ctx.http, format!("{} challenges added.", rows_count))
        .await?;

    Ok(())
}

#[command]
#[num_args(1)]
/// Create a new event
pub async fn create(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let name = args.single_quoted::<String>().unwrap_or("".to_string());

    if name.is_empty() {
        msg.channel_id
            .say(&ctx.http, "Must specify a name: !event create <name>")
            .await?;

        return Ok(());
    }

    let data = ctx.data.read().await;
    let pool = data
        .get::<PostgresPool>()
        .expect("Expected PostgresPool in TypeMap.");
    if let Ok(rows_created) = Event::create(pool, &name).await {
        // not sure it can ever go to the else clause, since any error inserting would return an
        // Error
        if rows_created > 0 {
            msg.channel_id
                .say(&ctx.http, format!("Created event '{}'", name))
                .await?;
        }
    } else {
        msg.channel_id
            .say(
                &ctx.http,
                "Could not create an event by that name. It already exists.",
            )
            .await?;
    }

    Ok(())
}
