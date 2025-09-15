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
/// This module is a simple cron equivalent. It spawns threads for the [`Task`]s that need to be completed.
mod scheduler;
/// A trait to define a job that needs to be executed regularly, for example checking for status updates daily.
mod tasks;
mod trace;
mod utils;

use crate::trace::setup_tracing;
use crate::trace::ReloadHandle;
use anyhow::Context as _;
use poise::{Context as PoiseContext, Framework, FrameworkOptions, PrefixFrameworkOptions};
use reaction_roles::handle_reaction;
use serenity::client::ClientBuilder;
use serenity::{
    all::{ReactionType, RoleId, UserId},
    client::{Context as SerenityContext, FullEvent},
    model::gateway::GatewayIntents,
};
use tracing::info;

use std::collections::{HashMap, HashSet};

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = PoiseContext<'a, Data, Error>;

/// The [`Data`] struct is kept in-memory by the Bot till it shutdowns and can be used to store session-persistent data.
struct Data {
    pub reaction_roles: HashMap<ReactionType, RoleId>,
    pub log_reload_handle: ReloadHandle,
}

impl Data {
    /// Returns a new [`Data`] with an empty `reaction_roles` field and the passed-in `reload_handle`.
    fn new_with_reload_handle(reload_handle: ReloadHandle) -> Self {
        Data {
            reaction_roles: HashMap::new(),
            log_reload_handle: reload_handle,
        }
    }
}

/// Struct to hold the resources necessary for the Discord bot to operate.
///
/// # Fields
///
/// * discord_token: The bot's discord token obtained from the Discord Developer Portal.
/// * owner_id: Used to allow access to privileged commands to specific users. Potential TODO: It would be more useful to allow access to certain roles (such as Moderator) in the Discord server instead. Poise already supports passing multiple IDs in the owner field when setting up the bot.
/// * prefix_string: The prefix used to issue commands to the bot on Discord.
#[derive(Default)]
struct BotConfig {
    discord_token: String,
    owner_id: UserId,
    prefix_string: String,
}

impl BotConfig {
    fn new_with_prefix(prefix_string: String) -> anyhow::Result<BotConfig> {
        let mut bot_config = BotConfig::default();
        bot_config
            .load_env_var()
            .context("Failed to load environment variables for BotConfig")?;
        bot_config.prefix_string = prefix_string;

        Ok(bot_config)
    }
    /// Loads [`BotConfig`]'s `discord_token` and `owner_id` fields from environment variables.
    ///
    /// Panics if any of the fields are not found in the env.
    fn load_env_var(&mut self) -> anyhow::Result<()> {
        self.discord_token =
            std::env::var("DISCORD_TOKEN").context("DISCORD_TOKEN was not found in env")?;
        self.owner_id = UserId::from(
            std::env::var("OWNER_ID")
                .context("OWNER_ID was not found in the env")?
                .parse::<u64>()
                .context("Failed to parse OWNER_ID")?,
        );

        Ok(())
    }
}

fn build_framework(owner_id: UserId, prefix_string: String, data: Data) -> Framework<Data, Error> {
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
            owners: HashSet::from([owner_id]),
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                scheduler::run_scheduler(ctx.clone()).await;
                Ok(data)
            })
        })
        .build()
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenv::dotenv().ok();
    let reload_handle = setup_tracing().context("Failed to setup tracing")?;
    info!("Tracing initialized. Continuing main...");

    let mut data = Data::new_with_reload_handle(reload_handle);
    data.populate_with_reaction_roles();

    let bot_config =
        BotConfig::new_with_prefix(String::from("$")).context("Failed to construct BotConfig")?;
    let framework = build_framework(bot_config.owner_id, bot_config.prefix_string, data);

    let mut client = ClientBuilder::new(
        bot_config.discord_token,
        GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT,
    )
    .framework(framework)
    .await
    .context("Failed to create the Serenity client")?;

    client
        .start()
        .await
        .context("Failed to start the Serenity client")?;

    info!("Starting amD...");

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
            handle_reaction(ctx, add_reaction, data, true).await;
        }
        FullEvent::ReactionRemove { removed_reaction } => {
            handle_reaction(ctx, removed_reaction, data, false).await;
        }
        _ => {}
    }

    Ok(())
}
