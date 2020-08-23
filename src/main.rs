mod commands;

use commands::{event::*, quest::*};
use serenity::{
    async_trait,
    client::Client,
    framework::standard::{macros::group, StandardFramework},
    model::gateway::Ready,
    prelude::{Context, EventHandler, TypeMapKey},
};
use sqlx::postgres::PgPoolOptions;

pub struct PostgresPool;
impl TypeMapKey for PostgresPool {
    type Value = sqlx::postgres::PgPool;
}

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
#[commands(add, archive, create, equest, set)]
struct Event;

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
                .group(&EVENT_GROUP)
                .group(&GENERAL_GROUP),
        )
        .await?;
    {
        let mut data = client.data.write().await;
        data.insert::<PostgresPool>(pool);
        println!("Connected to Postgres.");
    }

    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }

    Ok(())
}
