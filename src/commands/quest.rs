use crate::PostgresPool;
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::channel::Message,
    prelude::Context,
};

pub const DEFAULT_QUESTS_NUM: i64 = 3;

pub struct Scenario {
    pub title: String,
    pub set_name: String,
}

pub fn format_quests_display(scenarios: &Vec<Scenario>) -> String {
    scenarios
        .iter()
        .enumerate()
        .map(|(i, scenario)| {
            format!(
                "**Quest {}**: {} from {}",
                i + 1,
                scenario.title,
                scenario.set_name
            )
        })
        .collect::<Vec<String>>()
        .join("\n")
}

#[command]
pub async fn quest(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let quantity = args.single::<i64>().unwrap_or(DEFAULT_QUESTS_NUM);
    let data = ctx.data.read().await;
    let pool = data
        .get::<PostgresPool>()
        .expect("Expected PostgresPool in TypeMap.");

    let scenarios = sqlx::query_as!(
        Scenario,
        r#"
SELECT scenarios.title, sets.name AS set_name
FROM scenarios, sets
WHERE scenarios.set_id = sets.id
ORDER BY RANDOM()
LIMIT $1;
"#,
        quantity
    )
    .fetch_all(pool)
    .await?;

    msg.channel_id
        .say(&ctx.http, format_quests_display(&scenarios))
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_formats_the_scenarios() {
        let scenarios = vec![
            Scenario {
                title: "Foo".to_string(),
                set_name: "Core".to_string(),
            },
            Scenario {
                title: "Bar".to_string(),
                set_name: "Core".to_string(),
            },
            Scenario {
                title: "Baz".to_string(),
                set_name: "Core".to_string(),
            },
        ];

        assert_eq!(
            format_quests_display(&scenarios),
            r#"**Quest 1**: Foo from Core
**Quest 2**: Bar from Core
**Quest 3**: Baz from Core"#
        );
    }
}
