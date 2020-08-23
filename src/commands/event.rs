use crate::commands::quest;
use crate::PostgresPool;
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::channel::Message,
    prelude::Context,
};
use sqlx::{postgres::PgPool, prelude::Done};
use std::time::Duration;

const SELECTION_TIMEOUT: u64 = 60;

/// Format collection into a 1-indexed joint String
fn format_collection<T: std::fmt::Display>(collection: &Vec<T>) -> String {
    let width = collection.len() / 10;
    collection
        .iter()
        .enumerate()
        .map(|(i, item)| format!("{:>width$}.) {}", i + 1, item, width = width))
        .collect::<Vec<String>>()
        .join("\n")
}

/// Ask to pick from a collection and return the id of the set
async fn pick_collection<'a, T>(
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
async fn pick_sets(ctx: &Context, msg: &Message) -> anyhow::Result<Option<i64>> {
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

struct Event {
    id: i64,
    name: String,
}

impl Event {
    /// Find the active Event
    async fn find_by_active(pool: &PgPool, active: bool) -> anyhow::Result<Option<Event>> {
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
    async fn find_by_archive(pool: &PgPool, archive: bool) -> anyhow::Result<Vec<Event>> {
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
    async fn create(pool: &PgPool, name: &str) -> anyhow::Result<u64> {
        Ok(
            sqlx::query!(r#"INSERT INTO events ( name ) VALUES ( $1 )"#, name)
                .execute(pool)
                .await?
                .rows_affected(),
        )
    }
}

struct Set {
    id: i64,
    name: String,
}

impl Set {
    pub async fn find_all(pool: &PgPool) -> anyhow::Result<Vec<Set>> {
        Ok(sqlx::query_as!(
            Set,
            r#"
SELECT id, name
FROM sets
"#,
        )
        .fetch_all(pool)
        .await?)
    }
}

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
/// Add scenerios to an event
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
            format_collection(&events.iter().map(|event| &event.name).collect()),
        )
        .await?;
    let event;
    if let Some(e) = pick_collection(ctx, msg, &events).await? {
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
        .timeout(Duration::from_secs(10))
        .await
    {
        if choice.content == "1" {
            if let Some(set_id) = pick_sets(ctx, msg).await? {
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
                        format_collection(
                            &scenarios.iter().map(|scenario| &scenario.title).collect(),
                        ),
                    )
                    .await?;

                if let Some(scenario) = pick_collection(ctx, msg, &scenarios).await? {
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
            if let Some(set_id) = pick_sets(ctx, msg).await? {
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
            format_collection(&events.iter().map(|event| &event.name).collect()),
        )
        .await?;

    if let Some(event) = pick_collection(ctx, msg, &events).await? {
        sqlx::query!(
            r#"
UPDATE events
SET active = false
WHERE active = true
"#
        )
        .execute(pool)
        .await?;

        sqlx::query!(
            r#"
UPDATE events
SET active = true
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
            format_collection(&events.iter().map(|event| &event.name).collect()),
        )
        .await?;

    if let Some(event) = pick_collection(ctx, msg, &events).await? {
        // make any active event inactive when archiving
        sqlx::query!(
            r#"
UPDATE events
SET active = false,
  archive = true
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
        let scenarios = sqlx::query!(
            r#"
SELECT scenarios.title, sets.name AS set_name, scenarios.code
FROM scenarios, events_scenarios, sets
WHERE scenarios.id = events_scenarios.scenario_id
  AND events_scenarios.event_id = $1
  AND scenarios.set_id = sets.id
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
                .say(&ctx.http, "No quests registered with this event.")
                .await?;
        } else {
            msg.channel_id
                .say(
                    &ctx.http,
                    format_collection(
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
