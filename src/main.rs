mod commands;
mod models;
mod utils;

use commands::{event::*, quest::*};
use serenity::{
    async_trait,
    client::Client,
    framework::standard::{macros::group, StandardFramework},
    model::{gateway::Ready, id::ChannelId},
    prelude::{Context, EventHandler},
};
use sqlx::postgres::PgPoolOptions;
use std::collections::HashSet;

struct Handler;
#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[group]
#[commands(quest)]
struct General;

#[group]
#[prefix = "event"]
#[commands(
    add, archive, ccomplete, checkout, cload, complete, cprogress, cquest, create, equest,
    progress, set
)]
struct Event;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[cfg(debug_assertions)]
    dotenv::dotenv().ok();

    let discord_token =
        std::env::var("DISCORD_TOKEN").expect("Please provide the env var DISCORD_TOKEN");
    let database_url =
        std::env::var("DATABASE_URL").expect("Please provide the env var DATABASE_URL");
    let channels = parse_channels(&std::env::var("DISCORD_CHANNELS").unwrap_or("".to_string()))
        .expect("Please use comma separated list of Channel Ids in the env var DISCORD_CHANNELS");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    let mut client = Client::new(discord_token)
        .event_handler(Handler)
        .framework(
            StandardFramework::new()
                .configure(|c| c.prefix("!").allowed_channels(channels))
                .group(&EVENT_GROUP)
                .group(&GENERAL_GROUP),
        )
        .await?;
    {
        let mut data = client.data.write().await;
        data.insert::<utils::PostgresPool>(pool);
        println!("Connected to Postgres.");
    }

    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }

    Ok(())
}

fn parse_channels(input: &str) -> anyhow::Result<HashSet<ChannelId>> {
    input
        .split(",")
        .map(|str| Ok(ChannelId(str.parse::<u64>()?)))
        .collect::<anyhow::Result<HashSet<ChannelId>>>()
}
