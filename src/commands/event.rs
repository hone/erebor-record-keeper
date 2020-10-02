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
    utils::MessageBuilder,
};
use sqlx::prelude::Done;
use std::collections::HashMap;

pub mod admin;

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
#[num_args(1)]
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

// Struct for rows in cquest
#[derive(Debug)]
struct ChallengeRow<'a> {
    name: &'a str,
    code: &'a str,
    description: Option<&'a str>,
}

#[command]
#[aliases("quest")]
#[usage = ""]
#[example = ""]
/// List uncompleted challenges
pub async fn cquest(ctx: &Context, msg: &Message) -> CommandResult {
    let quest_count = 3;
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

    let rows = sqlx::query!(
        r#"
WITH completed_challenges AS (
        SELECT challenges.id, challenges.scenario_id
        FROM challenges_events_users, users, challenges_events, challenges
        WHERE challenges_events_users.user_id = users.id
            AND users.discord_id = $3
            AND challenges_events_users.challenges_events_id = challenges_events.id
            AND challenges_events.event_id = $1
            AND challenges_events.challenge_id = challenges.id
            AND 'Gauntlet' <> ALL (challenges.attributes)
),
completed_challenges_by_scenarios AS (
        SELECT scenario_id AS id, COUNT(id) AS count
        FROM completed_challenges
        GROUP BY scenario_id
),
challenge_count AS (
        SELECT challenges.scenario_id AS id, COUNT(challenges.id) AS count
        FROM challenges_events, challenges
        WHERE challenges_events.event_id = $1
            AND challenges_events.challenge_id = challenges.id
            AND 'Gauntlet' <> ALL (challenges.attributes)
        GROUP BY challenges.scenario_id
),
completed_scenarios AS (
    SELECT completed_challenges_by_scenarios.id
    FROM completed_challenges_by_scenarios, challenge_count
    WHERE challenge_count.id = completed_challenges_by_scenarios.id
        AND challenge_count.count = completed_challenges_by_scenarios.count
),
chosen_scenarios AS (
    SELECT scenarios.id, scenarios.title
    FROM scenarios, events_scenarios
    WHERE events_scenarios.event_id = $1
        AND events_scenarios.scenario_id = scenarios.id
        AND scenarios.id NOT IN (
            SELECT id
            FROM completed_scenarios
        )
ORDER BY RANDOM()
LIMIT $2
)

SELECT challenges.name, challenges.code, challenges.description, chosen_scenarios.title
FROM chosen_scenarios, challenges_events, challenges
WHERE challenges_events.event_id = $1
    AND challenges_events.challenge_id = challenges.id
    AND challenges.scenario_id = chosen_scenarios.id
    AND 'Gauntlet' <> ALL (challenges.attributes)
    AND challenges.id NOT IN (
        SELECT id
        FROM completed_challenges
    )
"#,
        event.id,
        quest_count,
        *msg.author.id.as_u64() as i64
    )
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        msg.channel_id
            .say(&ctx.http, "No challenges found.")
            .await?;

        return Ok(());
    }

    let mut scenarios: HashMap<&str, Vec<ChallengeRow>> = HashMap::new();

    for row in rows.iter() {
        scenarios.entry(&row.title).or_insert(Vec::new());
        let value = scenarios.get_mut(&row.title.as_str()).unwrap();
        value.push(ChallengeRow {
            name: &row.name,
            code: &row.code,
            description: row.description.as_ref().map(|a| a.as_str()),
        });
    }

    let mut content = MessageBuilder::new();
    let width = scenarios.len() / 10;
    for (i, (scenario, challenges)) in scenarios.iter().enumerate() {
        content.push(format!("{:>width$}.) {}\n", i + 1, scenario, width = width));
        for challenge in challenges.iter() {
            content.push(format!(
                "- (Code: **{}**) *{}* - {}\n",
                challenge.code,
                challenge.name,
                challenge.description.unwrap_or_else(|| "")
            ));
        }
    }

    msg.channel_id.say(&ctx.http, content.build()).await?;

    Ok(())
}

#[command]
#[min_args(1)]
#[aliases("complete")]
#[usage = "<challenge code>"]
#[example = "CON1901"]
/// Mark a challenge as complete
pub async fn ccomplete(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let code = match args.single::<String>() {
        Ok(code) => code,
        Err(_) => {
            utils::check_msg(
                msg.channel_id
                    .say(
                        &ctx.http,
                        "Requires an argument: !event ccomplete <challenge code>",
                    )
                    .await,
            );

            return Ok(());
        }
    };
    let data = ctx.data.read().await;
    let pool = data
        .get::<PostgresPool>()
        .expect("Expected PostgresPool in TypeMap.");

    let event = match Event::find_by_active(&pool, true).await? {
        Some(event) => event,
        None => {
            utils::check_msg(
                msg.channel_id
                    .say(&ctx.http, "There is no active event.")
                    .await,
            );

            return Ok(());
        }
    };
    let challenge_event = sqlx::query!(
        r#"
SELECT challenges_events.id, challenges.name
FROM challenges, challenges_events, events
WHERE challenges.code = $1
    AND events.id = $2
    AND challenges_events.challenge_id = challenges.id
    AND challenges_events.event_id = events.id
    "#,
        code,
        event.id
    )
    .fetch_all(pool)
    .await?
    .pop();

    let challenge_event = match challenge_event {
        Some(challenge_event) => challenge_event,
        None => {
            utils::check_msg(
                msg.channel_id
                    .say(&ctx.http, "Could not find a challenge by that code.")
                    .await,
            );

            return Ok(());
        }
    };

    let mut reply = MessageBuilder::new();
    let mut mentioned_users: Vec<&serenity::model::user::User> =
        msg.mentions.iter().map(|u| u).collect();
    mentioned_users.insert(0, &msg.author);
    let mut completed_users: Vec<Option<&serenity::model::user::User>> = Vec::new();

    for discord_user in mentioned_users.iter() {
        let user = User::find_or_create(pool, discord_user.id.as_u64(), &discord_user.name).await?;
        let row_count = sqlx::query!(
            r#"
INSERT INTO challenges_events_users ( challenges_events_id, user_id )
VALUES ( $1, $2 )
ON CONFLICT DO NOTHING
"#,
            challenge_event.id,
            user.id
        )
        .execute(pool)
        .await?
        .rows_affected();

        completed_users.push(if row_count > 0 {
            Some(discord_user)
        } else {
            None
        });
    }

    let completed_users: Vec<_> = completed_users
        .into_iter()
        .filter_map(|user| user)
        .collect();
    if completed_users.is_empty() {
        for discord_user in mentioned_users.iter() {
            reply.mention(*discord_user);
            reply.push(" ");
        }
        reply.push(format!(
            "All users have already completed challenge '{}'",
            challenge_event.name
        ));
    } else {
        for discord_user in completed_users.iter() {
            reply.mention(*discord_user);
            reply.push(" ");
        }
        reply.push(format!(
            "You completed challenge '{}'",
            challenge_event.name
        ));
    }

    utils::check_msg(msg.channel_id.say(&ctx.http, &reply.build()).await);

    Ok(())
}

#[command]
#[aliases("progress")]
#[usage = ""]
#[example = ""]
/// Display your challenge progress
pub async fn cprogress(ctx: &Context, msg: &Message) -> CommandResult {
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

    let challenge_count = sqlx::query!(
        r#"
SELECT COUNT(*)
FROM challenges_events, challenges
WHERE challenges_events.event_id = $1
    AND challenges_events.challenge_id = challenges.id
    AND 'Gauntlet' <> ALL (challenges.attributes)
"#,
        event.id
    )
    .fetch_one(pool)
    .await?
    .count
    .unwrap();

    let completed_challenges = sqlx::query!(
        r#"
SELECT challenges.name, challenges.code, challenges.description, scenarios.title
FROM challenges_events_users, challenges_events, users, challenges, scenarios
WHERE challenges_events_users.challenges_events_id = challenges_events.id
    AND challenges_events.event_id = $1
    AND challenges_events_users.user_id = users.id
    AND users.discord_id = $2
    AND challenges_events.challenge_id = challenges.id
    AND challenges.scenario_id = scenarios.id
    AND 'Gauntlet' <> ALL (challenges.attributes)
"#,
        event.id,
        *msg.author.id.as_u64() as i64
    )
    .fetch_all(pool)
    .await?;

    msg.reply(
        &ctx.http,
        format!(
            "You've completed {} of {} total challenges: {:.2}%",
            completed_challenges.len(),
            challenge_count,
            (completed_challenges.len() as f32 / challenge_count as f32) * 100.0
        ),
    )
    .await?;

    Ok(())
}

#[command]
#[usage = ""]
#[example = ""]
/// Display a random challenge from the gauntlet
pub async fn gauntlet(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    let pool = data
        .get::<PostgresPool>()
        .expect("Expected PostgresPool in TypeMap.");

    let event = match Event::find_by_active(pool, true).await? {
        Some(event) => event,
        None => {
            msg.channel_id
                .say(&ctx.http, "No active event set.")
                .await?;

            return Ok(());
        }
    };

    let result = sqlx::query!(
        r#"
SELECT name, description
FROM challenges, challenges_events
WHERE challenges_events.event_id = $1
    AND challenges_events.challenge_id = challenges.id
    AND 'Gauntlet' = ANY (attributes)
ORDER BY RANDOM()
LIMIT 1
"#,
        event.id
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
        "Could not find any gauntlet challenges.".to_string()
    };

    msg.reply(&ctx.http, message).await?;

    Ok(())
}
