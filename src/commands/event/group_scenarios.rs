//! Collection of commands for Events with scenario based group wide completion

use crate::{
    commands::quest,
    models::{event::Event, user::User},
    utils::{self, PostgresPool},
};
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::channel::Message,
    prelude::Context,
};

#[command]
#[min_args(0)]
#[max_args(1)]
#[aliases(quest)]
#[usage = "<quantity=default:3>"]
#[example = ""]
#[example = "5"]
/// Return a list of quests remaining for the event
pub async fn equest(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let quantity = args.single::<i64>().unwrap_or(quest::DEFAULT_QUESTS_NUM);
    let data = ctx.data.read().await;
    let pool = data
        .get::<PostgresPool>()
        .expect("Expected PostgresPool in TypeMap.");

    if let Some(event) = Event::find_by_active(pool, true).await? {
        let scenarios = sqlx::query!(
            r#"
SELECT scenarios.title, sets.name AS set_name, scenarios.code
FROM scenarios, events_scenarios, sets
WHERE scenarios.id = events_scenarios.scenario_id
    AND events_scenarios.event_id = $1
    AND scenarios.set_id = sets.id
    AND events_scenarios.complete = false
    AND (events_scenarios.checkout IS NULL OR events_scenarios.checkout < CURRENT_TIMESTAMP - INTERVAL '2 hours')
ORDER BY RANDOM()
LIMIT $2
"#,
            event.id,
            quantity,
        )
        .fetch_all(pool)
        .await?;

        if scenarios.is_empty() {
            msg.channel_id
                .say(&ctx.http, "No more quests registered with this event.")
                .await?;
        } else {
            msg.channel_id
                .say(
                    &ctx.http,
                    utils::format_collection(
                        &scenarios
                            .iter()
                            .map(|scenario| {
                                format!(
                                    "{} from {} **with Code**: {}",
                                    scenario.title, scenario.set_name, scenario.code
                                )
                            })
                            .collect(),
                    ),
                )
                .await?;
        }
    } else {
        msg.channel_id
            .say(&ctx.http, "No active event found.")
            .await?;
    }

    Ok(())
}

#[command]
#[min_args(1)]
#[usage = "<scenario code>"]
#[example = "0101"]
/// Mark a scenario as complete
pub async fn complete(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    if let Ok(code) = args.single::<String>() {
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

        let scenario;
        if let Ok(s) = sqlx::query!(
            r#"
SELECT id, title
FROM scenarios
WHERE code = $1
"#,
            code
        )
        .fetch_one(pool)
        .await
        {
            scenario = s;
        } else {
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("No scenario found by that code: {}", code),
                )
                .await?;

            return Ok(());
        }

        let row_count = sqlx::query!(
            r#"
UPDATE events_scenarios
SET complete = true,
    updated_at = CURRENT_TIMESTAMP
WHERE event_id = $1
    AND scenario_id = $2
"#,
            event.id,
            scenario.id,
        )
        .execute(pool)
        .await?
        .rows_affected();

        // Not sure this else can be triggered
        if row_count > 0 {
            msg.channel_id
                .say(&ctx.http, format!("Completed Quest: {}", scenario.title))
                .await?;
        }
    }

    Ok(())
}

#[command]
#[num_args(1)]
#[usage = "<scenario code>"]
#[example = "0101"]
/// Claim a Quest for 2 hours. This will hide it from the equest command.
pub async fn checkout(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    if let Ok(code) = args.single::<String>() {
        let data = ctx.data.read().await;
        let pool = data
            .get::<PostgresPool>()
            .expect("Expected PostgresPool in TypeMap.");

        let event;
        if let Some(e) = Event::find_by_active(pool, true).await? {
            event = e;
        } else {
            msg.channel_id
                .say(&ctx.http, "No active event found.")
                .await?;

            return Ok(());
        }

        let scenario;
        if let Ok(s) = sqlx::query!(
            r#"
SELECT id, title
FROM scenarios
WHERE code = $1
"#,
            code
        )
        .fetch_one(pool)
        .await
        {
            scenario = s;
        } else {
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("No scenario found by that code: {}", code),
                )
                .await?;

            return Ok(());
        }

        let user = User::find_or_create(pool, msg.author.id.as_u64(), &msg.author.name).await?;
        if let Ok(result) = sqlx::query!(
            r#"
UPDATE events_scenarios
SET checkout = CURRENT_TIMESTAMP,
    checkout_user_id = $1,
    updated_at = CURRENT_TIMESTAMP
WHERE event_id = $2
    AND scenario_id = $3
    AND (checkout IS NULL OR checkout < CURRENT_TIMESTAMP - INTERVAL '2 hours')
"#,
            user.id,
            event.id,
            scenario.id,
        )
        .execute(pool)
        .await
        {
            if result.rows_affected() > 0 {
                msg.channel_id
                    .say(&ctx.http, format!("Reserving Quest **{}**", scenario.title))
                    .await?;
            } else {
                msg.channel_id
                    .say(
                        &ctx.http,
                        format!("Quest **{}** is already reserved.", scenario.title),
                    )
                    .await?;
            }
        }
    }

    Ok(())
}

#[command]
#[usage = ""]
#[example = ""]
/// Display progress for scenarios finished for the group
pub async fn progress(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    let pool = data
        .get::<PostgresPool>()
        .expect("Expected PostgresPool in TypeMap.");

    if let Some(event) = Event::find_by_active(pool, true).await? {
        let calc = sqlx::query!(
            r#"
SELECT (cnt/total::float)*100 AS perc
FROM (
    SELECT COUNT(*) AS total,
        SUM(CASE WHEN complete = true THEN 1 ELSE 0 END) AS cnt
    FROM events_scenarios
    WHERE event_id = $1
) x
"#,
            event.id
        )
        .fetch_one(pool)
        .await?;

        if let Some(perc) = calc.perc {
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("The fellowship has completed {:.2}% of quests.", perc),
                )
                .await?;
        } else {
            msg.channel_id
                .say(&ctx.http, "Could not calculate progress.")
                .await?;
        }
    }

    Ok(())
}
