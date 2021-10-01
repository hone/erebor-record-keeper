use crate::{
    models::{event::Event, user::User},
    utils::{self, PostgresPool},
};
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::channel::Message,
    prelude::Context,
    utils::MessageBuilder,
};
use std::collections::HashMap;

pub mod admin;
pub mod group_challenges;
pub mod group_scenarios;
pub mod kang;

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
#[aliases(all)]
#[usage = ""]
#[example = ""]
/// List all available challenges
pub async fn call(ctx: &Context, msg: &Message) -> CommandResult {
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
SELECT challenges.name, challenges.code, challenges.description, scenarios.title
FROM events_scenarios, challenges_events, challenges, scenarios
WHERE events_scenarios.event_id = $1
    AND events_scenarios.scenario_id = scenarios.id
    AND challenges_events.event_id = events_scenarios.event_id
    AND challenges_events.challenge_id = challenges.id
    AND challenges.scenario_id = scenarios.id
    AND 'Gauntlet' <> ALL (challenges.attributes)
"#,
        event.id
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

    let width = scenarios.len() / 10;
    for (i, (scenario, challenges)) in scenarios.iter().enumerate() {
        let mut content = MessageBuilder::new();

        content.push(format!("{:>width$}.) {}\n", i + 1, scenario, width = width));
        for challenge in challenges.iter() {
            content.push(format!(
                "- (Code: **{}**) *{}* - {}\n",
                challenge.code,
                challenge.name,
                challenge.description.unwrap_or_else(|| "")
            ));
        }

        msg.channel_id.say(&ctx.http, content.build()).await?;
    }

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
#[aliases("my-progress")]
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

    let all_challenges = event.find_all_challenges(&pool).await?;
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
            all_challenges.len(),
            (completed_challenges.len() as f32 / all_challenges.len() as f32) * 100.0
        ),
    )
    .await?;

    Ok(())
}
