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

const KANG_DEFAULT_MODE: &str = "all";

fn valid_mode(mode: impl AsRef<str>) -> bool {
    if !vec!["standard", "expert", "all"].contains(&mode.as_ref().to_lowercase().as_str()) {
        false
    } else {
        true
    }
}

struct KangChallenge {
    name: String,
    code: String,
}

#[command]
#[min_args(0)]
#[max_args(2)]
#[aliases(all)]
#[usage = "<mode=default:all>"]
/// Return a list of every Kang in the challenge
/// supported modes: all, standard, expert
pub async fn kall(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let mode = args
        .single::<String>()
        .unwrap_or(String::from(KANG_DEFAULT_MODE));
    if !valid_mode(&mode) {
        msg.channel_id
            .say(
                &ctx.http,
                "The only valid modes are 'standard', 'expert', 'all'.",
            )
            .await?;
        return Ok(());
    }
    let data = ctx.data.read().await;
    let pool = data
        .get::<PostgresPool>()
        .expect("Expected PostgresPool in TypeMap.");

    let challenges: Vec<KangChallenge> = if mode == "all" {
        sqlx::query!(
            r#"
SELECT name, code
FROM challenges
WHERE 'Council of 100 Kangs'=ANY(attributes)
"#,
        )
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|record| KangChallenge {
            name: record.name,
            code: record.code,
        })
        .collect()
    } else {
        sqlx::query!(
            r#"
SELECT name, code
FROM challenges
WHERE 'LeacgueCG Con 2021: The Return of the Kang'=ANY(attributes)
    AND 'Council of 100 Kangs'=ANY(attributes)
    AND INITCAP($1)=ANY(attributes)
"#,
            mode,
        )
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|record| KangChallenge {
            name: record.name,
            code: record.code,
        })
        .collect()
    };

    if challenges.is_empty() {
        msg.channel_id
            .say(&ctx.http, "No Kangs could be found for that query.")
            .await?;
    } else {
        for message in utils::format_large_collection(
            &challenges
                .iter()
                .map(|challenge| format!("**{}** with Code: *{}*", challenge.name, challenge.code))
                .collect(),
        ) {
            msg.channel_id.say(&ctx.http, message).await?;
        }
    }

    Ok(())
}

#[command]
#[min_args(0)]
#[max_args(2)]
#[usage = "<mode=default:all> <quantity=default:3>"]
/// Get some Kangs to fight!
/// supported modes: all, standard, expert
pub async fn mission(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let mode = args
        .single::<String>()
        .unwrap_or(String::from(KANG_DEFAULT_MODE));
    let quantity = args.single::<i64>().unwrap_or(quest::DEFAULT_QUESTS_NUM);
    if !valid_mode(&mode) {
        msg.channel_id
            .say(
                &ctx.http,
                "The only valid modes are 'standard', 'expert', 'all'.",
            )
            .await?;
        return Ok(());
    }
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

    let challenges: Vec<KangChallenge> = if mode == "all" {
        sqlx::query!(
            r#"
SELECT name, code
FROM challenges, challenges_events
WHERE challenges_events.event_id = $1
    AND challenges.id = challenges_events.challenge_id
    AND 'Council of 100 Kangs'=ANY(challenges.attributes)
    AND challenges_events.id NOT IN (
        SELECT challenges_events_id
        FROM challenges_events_users
    )
ORDER BY RANDOM()
LIMIT $2
"#,
            event.id,
            quantity,
        )
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|challenge| KangChallenge {
            name: challenge.name,
            code: challenge.code,
        })
        .collect()
    } else {
        sqlx::query!(
            r#"
SELECT name, code
FROM challenges, challenges_events
WHERE challenges_events.event_id = $1
    AND challenges.id = challenges_events.challenge_id
    AND 'Council of 100 Kangs'=ANY(challenges.attributes)
    AND INITCAP($2)=ANY(challenges.attributes)
    AND challenges_events.id NOT IN (
        SELECT challenges_events_id
        FROM challenges_events_users
    )
ORDER BY RANDOM()
LIMIT $3
"#,
            event.id,
            mode,
            quantity,
        )
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|challenge| KangChallenge {
            name: challenge.name,
            code: challenge.code,
        })
        .collect()
    };

    if challenges.is_empty() {
        msg.channel_id
            .say(&ctx.http, "Couldn't find any Kangs.")
            .await?;
    } else {
        msg.channel_id
            .say(
                &ctx.http,
                utils::format_collection(
                    &challenges
                        .iter()
                        .map(|challenge| {
                            format!("**{}** with Code: *{}*", challenge.name, challenge.code)
                        })
                        .collect(),
                ),
            )
            .await?;
    }

    Ok(())
}

#[command]
#[num_args(0)]
#[aliases(progress)]
#[usage = ""]
/// Check on progress
pub async fn kprogress(ctx: &Context, msg: &Message) -> CommandResult {
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

    let calc = sqlx::query!(
        r#"
WITH completed AS (
    SELECT COUNT(DISTINCT(challenges_events_users.challenges_events_id)) as cnt
    FROM challenges_events_users, challenges_events, challenges
    WHERE challenges_events.event_id = $1
        AND challenges_events.id = challenges_events_users.challenges_events_id
        AND challenges.id = challenges_events.challenge_id
        AND 'Council of 100 Kangs'=ANY(challenges.attributes)
    ), total AS (
    SELECT COUNT(challenges_events.id) AS cnt
    FROM challenges_events, challenges
    WHERE challenges_events.event_id = $1
        AND challenges.id = challenges_events.challenge_id
        AND 'Council of 100 Kangs'=ANY(challenges.attributes)
    )
SELECT (completed.cnt/total.cnt::float)*100 AS perc
FROM completed, total
"#,
        event.id
    )
    .fetch_one(pool)
    .await?;

    if let Some(perc) = calc.perc {
        msg.channel_id
            .say(
                &ctx.http,
                format!("The heroes have completed {:.2}% of Kangs.", perc),
            )
            .await?;
    } else {
        msg.channel_id
            .say(&ctx.http, "Could not calculate progress.")
            .await?;
    }

    Ok(())
}

#[command]
#[min_args(1)]
#[aliases(complete)]
#[usage = "<code>"]
#[example = "ROTK2021-MCS36"]
/// Conquer Kang!
pub async fn conquer(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let code = match args.single::<String>() {
        Ok(code) => code,
        Err(_) => {
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("This command taske a code as an argument."),
                )
                .await?;

            return Ok(());
        }
    };
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
    let challenge_event = match sqlx::query!(
        r#"
SELECT challenges.name, challenges_events.id
FROM challenges, challenges_events
WHERE challenges_events.event_id = $1
    AND challenges.code = $2
    AND challenges_events.challenge_id = challenges.id
    AND 'Council of 100 Kangs'=ANY(challenges.attributes)
"#,
        event.id,
        code
    )
    .fetch_one(pool)
    .await
    {
        Ok(challenge) => challenge,
        Err(_) => {
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("No scenario found by that code: {}", code),
                )
                .await?;

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
