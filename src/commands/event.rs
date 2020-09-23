use crate::utils::PostgresPool;
use crate::{
    commands::quest,
    models::{event::Event, user::User},
    utils,
};
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::channel::Message,
    prelude::Context,
};
use sqlx::prelude::Done;
use std::time::Duration;

// TODO set permissions on commands

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
#[min_args(0)]
#[max_args(1)]
#[aliases(quest)]
/// Return a list of quests associated with the event
pub async fn equest(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let quantity = args.single::<i64>().unwrap_or(quest::DEFAULT_QUESTS_NUM);
    let data = ctx.data.read().await;
    let pool = data
        .get::<PostgresPool>()
        .expect("Expected PostgresPool in TypeMap.");

    if let Some(event) = Event::find_by_active(pool, true).await? {
        let times = sqlx::query!(r#"
SELECT cast(extract(epoch from current_timestamp) AS integer) AS current, cast(extract(epoch from CURRENT_TIMESTAMP - INTERVAL '1 minutes') AS integer) AS interval, cast(extract(epoch from events_scenarios.checkout) AS integer) AS checkout
FROM events_scenarios
WHERE events_scenarios.event_id = $1
"#, event.id).fetch_all(pool).await?;
        for time in times.iter() {
            println!(
                "{} | {} | {}",
                time.current.unwrap_or(0),
                time.interval.unwrap_or(0),
                time.checkout.unwrap_or(0)
            );
        }
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
#[num_args(1)]
/// Complete a quest
pub async fn complete(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
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
/// Checkout a Quest
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
/// Display how much of the event quests are complete
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
