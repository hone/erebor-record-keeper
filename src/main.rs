use erebor_record_keeper::hob_scenario_parser::{fetch, Scenario};
use serenity::{
    async_trait,
    client::Client,
    framework::standard::{
        macros::{command, group},
        Args, CommandResult, StandardFramework,
    },
    model::{channel::Message, gateway::Ready},
    prelude::{Context, EventHandler, TypeMapKey},
};
use sqlx::postgres::PgPoolOptions;

const DEFAULT_QUESTS_NUM: i64 = 3;

struct PostgresPool;
impl TypeMapKey for PostgresPool {
    type Value = sqlx::postgres::PgPool;
}

struct QuestData;
impl TypeMapKey for QuestData {
    type Value = Vec<Scenario>;
}

struct Handler;
#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[group]
#[commands(ping, quest)]
struct General;

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "Pong!").await?;

    Ok(())
}

#[command]
async fn quest(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let quantity = args.single::<i64>().unwrap_or(DEFAULT_QUESTS_NUM);
    let data = ctx.data.read().await;
    let pool = data
        .get::<PostgresPool>()
        .expect("Expected PgPoolOptions in TypeMap.");
    let scenarios = sqlx::query!(
        r#"
SELECT scenarios.title, sets.name AS set
FROM scenarios, sets
WHERE scenarios.set_id = sets.id
ORDER BY RANDOM()
LIMIT $1;
"#,
        quantity
    )
    .fetch_all(pool)
    .await?;

    for (i, scenario) in scenarios.iter().enumerate() {
        msg.channel_id
            .say(
                &ctx.http,
                format!(
                    "**Quest {}**: {} from {}",
                    i + 1,
                    scenario.title,
                    scenario.set,
                ),
            )
            .await?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(debug_assertions)]
    dotenv::dotenv().ok();

    let discord_token =
        std::env::var("DISCORD_TOKEN").expect("Please provide the env var DISCORD_TOKEN");
    let database_url =
        std::env::var("DATABASE_URL").expect("Please provide the env var DATABASE_URL");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    let mut client = Client::new(discord_token)
        .event_handler(Handler)
        .framework(
            StandardFramework::new()
                .configure(|c| c.prefix("!"))
                .group(&GENERAL_GROUP),
        )
        .await?;
    {
        let scenarios = fetch().await.unwrap();
        println!("Loading all {} scenarios", scenarios.len());
        let mut data = client.data.write().await;
        data.insert::<PostgresPool>(pool);
        data.insert::<QuestData>(scenarios);
    }

    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }

    Ok(())
}
