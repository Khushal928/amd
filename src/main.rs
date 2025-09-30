/*
amFOSS Daemon: A discord bot for the amFOSS Discord server.
Copyright (C) 2024 amFOSS

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/
mod commands;
mod graphql;
mod ids;
mod reaction_roles;
mod scheduler;
mod tasks;
mod trace;
mod utils;

use anyhow::Context as _;
use graphql::GraphQLClient;
use poise::{Context as PoiseContext, Framework, FrameworkOptions, PrefixFrameworkOptions};
use reaction_roles::handle_reaction;
use reqwest::Client;
use serenity::client::ClientBuilder;
use serenity::{
    all::{ReactionType, RoleId, UserId},
    client::{Context as SerenityContext, FullEvent},
    model::gateway::GatewayIntents,
};
use trace::{setup_tracing, ReloadHandle};
use tracing::{info, instrument};

use std::collections::HashMap;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = PoiseContext<'a, Data, Error>;

/// The [`Data`] struct is kept in-memory by the Bot till it shutdowns and can be used to store session-persistent data.
#[derive(Clone)]
struct Data {
    reaction_roles: HashMap<ReactionType, RoleId>,
    log_reload_handle: ReloadHandle,
    graphql_client: GraphQLClient,
}

impl Data {
    /// Returns a new [`Data`] with an empty `reaction_roles` field and the passed-in `reload_handle`.
    fn new(reload_handle: ReloadHandle, root_url: String) -> Self {
        Data {
            reaction_roles: HashMap::new(),
            log_reload_handle: reload_handle,
            graphql_client: GraphQLClient::new(root_url),
        }
    }
}

/// Builds a [`poise::Framework`] with the given arguments and commands from [`commands::get_commands`].
#[instrument(level = "debug", skip(data))]
fn build_framework(
    owners: Option<UserId>,
    prefix_string: String,
    data: Data,
) -> Framework<Data, Error> {
    Framework::builder()
        .options(FrameworkOptions {
            commands: commands::get_commands(),
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler(ctx, event, framework, data))
            },
            prefix_options: PrefixFrameworkOptions {
                prefix: Some(prefix_string),
                ..Default::default()
            },
            owners: owners.into_iter().collect(),
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                scheduler::run_scheduler(ctx.clone(), data.graphql_client.clone()).await;
                Ok(data)
            })
        })
        .build()
}

/// Environment variables for amD
///
/// # Fields
///
/// * debug: a boolean flag that decides in what context the application will be running on. When true, it is assumed to be in development. This allows us to filter out logs from `stdout` when in production. Defaults to false if not set.
/// * enable_debug_libraries: Boolean flag that controls whether tracing will output logs from other crates used in the project. This is only needed for really serious bugs. Defaults to false if not set.
/// * discord_token: The bot's discord token obtained from the Discord Developer Portal. The only mandatory variable required.
/// * owner_id: Used to allow access to privileged commands to specific users. If not passed, will set the bot to have no owners.
/// * prefix_string: The prefix used to issue commands to the bot on Discord. Always set to "$".
struct Config {
    debug: bool,
    enable_debug_libraries: bool,
    discord_token: String,
    owner_id: Option<UserId>,
    prefix_string: String,
    root_url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            debug: parse_bool_env("DEBUG"),
            enable_debug_libraries: parse_bool_env("ENABLE_DEBUG_LIBRARIES"),
            discord_token: std::env::var("DISCORD_TOKEN")
                .expect("DISCORD_TOKEN was not found in env"),
            owner_id: parse_owner_id_env("OWNER_ID"),
            prefix_string: String::from("$"),
            root_url: std::env::var("ROOT_URL").expect("ROOT_URL was not found in env"),
        }
    }
}

/// Tries to access the environment variable through the key passed in. If it is set, it will try to parse it as u64 and if that fails, it will log the error and return the default value None. If it suceeds the u64 parsing, it will convert it to a UserId and return Some(UserId). If the env. var. is not set, it will return None.
fn parse_owner_id_env(key: &str) -> Option<UserId> {
    std::env::var(key)
        .ok()
        .and_then(|s| {
            s.parse::<u64>()
                .map_err(|_| eprintln!("WARNING: Invalid OWNER_ID value '{}', ignoring.", s))
                .ok()
        })
        .map(UserId::new)
}

/// Tries to access the environment variable through the key passed in. If it is set but an invalid boolean, it will log an error through tracing and default to false. If it is not set, it will default to false.
fn parse_bool_env(key: &str) -> bool {
    std::env::var(key)
        .map(|val| {
            val.parse().unwrap_or_else(|_| {
                eprintln!(
                    "Warning: Invalid DEBUG value '{}', defaulting to false",
                    val
                );
                false
            })
        })
        .unwrap_or(false)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenv::dotenv().ok();

    let config = Config::default();
    let reload_handle = setup_tracing(config.debug, config.enable_debug_libraries)
        .context("Failed to setup tracing")?;

    let mut data = Data::new(reload_handle, config.root_url);
    data.populate_with_reaction_roles();

    let framework = build_framework(config.owner_id, config.prefix_string, data);

    let mut client = ClientBuilder::new(
        config.discord_token,
        GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT,
    )
    .framework(framework)
    .await
    .context("Failed to create the Serenity client")?;

    info!("Starting amD...");

    client
        .start()
        .await
        .context("Failed to start the Serenity client")?;

    Ok(())
}

async fn event_handler(
    ctx: &SerenityContext,
    event: &FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        FullEvent::ReactionAdd { add_reaction } => {
            handle_reaction(ctx, add_reaction, data, true).await?;
        }
        FullEvent::ReactionRemove { removed_reaction } => {
            handle_reaction(ctx, removed_reaction, data, false).await?;
        }
        _ => {}
    }

    Ok(())
}
