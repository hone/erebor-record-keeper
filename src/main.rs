mod commands;
mod models;
mod utils;

use commands::{
    challenge::*,
    event::{admin::*, kang::*, *},
    quest::*,
};
use serenity::{
    async_trait,
    client::Client,
    framework::standard::{
        help_commands,
        macros::{group, help},
        Args, CommandGroup, CommandResult, HelpOptions, StandardFramework,
    },
    model::{
        gateway::Ready,
        id::ChannelId,
        prelude::{Message, UserId},
    },
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
#[commands(gauntlet, quest)]
struct General;

#[group]
#[prefix = "event"]
#[description = "Set of Commands for interacting with an event."]
#[sub_groups("EventAdmin", "EventKang")]
#[commands(call, ccomplete, cprogress, cquest)]
struct Event;

#[group]
#[prefix = "admin"]
#[allowed_roles("Tech Team")]
#[commands(add, archive, cload, create, set)]
struct EventAdmin;

#[group]
#[prefix = "kang"]
#[description = "Set of Commands for the Return of the Kang Challenge!"]
#[commands(conquer, kall, kprogress, mission)]
struct EventKang;

#[help]
#[individual_command_tip = "If you want more information about a specific command, just pass the command as argument."]
#[lacking_role("hide")]
#[max_levenshtein_distance(3)]
async fn my_help(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
    Ok(())
}

#[tokio::main]
async fn main() {
    #[cfg(debug_assertions)]
    dotenv::dotenv().ok();

    let discord_token =
        std::env::var("DISCORD_TOKEN").expect("Please provide the env var DISCORD_TOKEN");
    let database_url =
        std::env::var("DATABASE_URL").expect("Please provide the env var DATABASE_URL");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Unable to connect to Postgres.");

    let mut client = Client::builder(discord_token)
        .event_handler(Handler)
        .framework(
            StandardFramework::new()
                .configure(|c| c.prefix("!"))
                .help(&MY_HELP)
                .group(&EVENT_GROUP)
                .group(&GENERAL_GROUP),
        )
        .await
        .expect("Error creating client.");
    {
        let mut data = client.data.write().await;
        data.insert::<utils::PostgresPool>(pool);
        println!("Connected to Postgres.");
    }

    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}
