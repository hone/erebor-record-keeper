use erebor_record_keeper::hob_scenario_parser::{fetch, Scenario};
use rand::{rngs::SmallRng, Rng, SeedableRng};
use serenity::{
    async_trait,
    client::Client,
    framework::standard::{
        macros::{command, group},
        CommandResult, StandardFramework,
    },
    model::{channel::Message, gateway::Ready},
    prelude::{Context, EventHandler, TypeMapKey},
};

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
async fn quest(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    let scenarios = data
        .get::<QuestData>()
        .expect("Expected QuestData in TypeMap.");
    let indexes = (0..3).fold(Vec::with_capacity(3), |mut indexes, _| {
        let index = random_index(&indexes, scenarios.len());
        indexes.push(index);

        indexes
    });
    for (i, &index) in indexes.iter().enumerate() {
        let scenario = &scenarios[index];

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

fn random_index(existing: &[usize], max: usize) -> usize {
    let mut rng = SmallRng::from_entropy();
    let mut index = max;

    while index >= max || existing.contains(&index) {
        index = rng.gen_range(0, max);
    }

    index
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(debug_assertions)]
    dotenv::dotenv().ok();

    let discord_token =
        std::env::var("DISCORD_TOKEN").expect("Please provide the env var DISCORD_TOKEN");

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
        data.insert::<QuestData>(scenarios);
    }

    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }

    Ok(())
}
