mod commands;
mod models;
mod utils;

use commands::{
    challenge::*,
    event::{admin::*, group_challenges::*, group_scenarios::*, kang::*, *},
    quest::*,
};
use serenity::{
    async_trait,
    client::Client,
    framework::standard::{
        help_commands,
        macros::{group, help, hook},
        Args, CommandGroup, CommandResult, HelpOptions, StandardFramework,
    },
    model::{
        gateway::Ready,
        prelude::{Message, UserId},
    },
    prelude::{Context, EventHandler},
};
use sqlx::postgres::PgPoolOptions;
use std::{collections::HashSet, process::exit};
use tracing::{error, info, instrument};

struct Handler;
#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
    }
}

#[group]
#[commands(gauntlet, quest)]
struct General;

#[group]
#[prefix = "event"]
#[description = "Set of Commands for interacting with an event."]
#[sub_groups("EventAdmin", "EventKang")]
#[commands(call, ccomplete, cgroup, cgroupall, cprogress, cquest)]
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

#[hook]
#[instrument]
async fn before_hook(_: &Context, msg: &Message, command_name: &str) -> bool {
    info!(command = command_name, user = msg.author.name.as_str());

    true
}

#[tokio::main]
#[instrument]
async fn main() {
    #[cfg(debug_assertions)]
    dotenv::dotenv().ok();

    tracing_subscriber::fmt::init();

    let discord_token =
        std::env::var("DISCORD_TOKEN").expect("Please provide the env var DISCORD_TOKEN");
    let database_url =
        std::env::var("DATABASE_URL").expect("Please provide the env var DATABASE_URL");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .unwrap_or_else(|_| {
            error!("Unable to connect to Postgres.");
            exit(1);
        });

    let mut client = Client::builder(discord_token)
        .event_handler(Handler)
        .framework(
            StandardFramework::new()
                .configure(|c| c.prefix("!"))
                .before(before_hook)
                .help(&MY_HELP)
                .group(&EVENT_GROUP)
                .group(&GENERAL_GROUP),
        )
        .await
        .unwrap_or_else(|_| {
            error!("Error creating client");
            exit(1);
        });
    {
        let mut data = client.data.write().await;
        data.insert::<utils::PostgresPool>(pool);
        info!("Connected to Postgres.");
    }

    if let Err(why) = client.start().await {
        error!("An error occurred while running the client: {:?}", why);
        exit(1);
    }
}
