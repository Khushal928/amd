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
mod utils;

use anyhow::Context as _;
use poise::{Context as PoiseContext, Framework, FrameworkOptions, PrefixFrameworkOptions};
use reaction_roles::{handle_reaction, populate_data_with_reaction_roles};
use serenity::{
    all::{ReactionType, RoleId, UserId},
    client::{Context as SerenityContext, FullEvent},
    model::gateway::GatewayIntents,
};
use tokio::sync::RwLock;
use tracing::info;
use tracing_subscriber::{
    fmt,
    layer::SubscriberExt,
    reload::{self, Layer},
    EnvFilter, Registry,
};

use std::{
    collections::{HashMap, HashSet},
    fs::File,
    sync::Arc,
};

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = PoiseContext<'a, Data, Error>;
pub type ReloadHandle = Arc<RwLock<reload::Handle<EnvFilter, Registry>>>;

pub struct Data {
    pub reaction_roles: HashMap<ReactionType, RoleId>,
    pub log_reload_handle: ReloadHandle,
}

/// Environment variables that our tracing configuration relies on
///
/// # Fields
///
/// * env: String that decides in what context the application will be running on i.e "production" or "development". This allows us to filter out logs from `stdout` when in production. Possible TODO: Could be replaced to a boolean `is_dev` or something similar to be more constrained than a string.
/// * enable_debug_libraries: Boolean flag that controls whether tracing will output logs from other crates used in the project. This is only needed for really serious bugs.
struct TracingConfig {
    env: String,
    enable_debug_libraries: bool,
}

impl TracingConfig {
    /// Encapsulate all the required env variables into a [`TracingConfig`]
    fn load_tracing_config() -> Self {
        Self {
            env: std::env::var("AMD_RUST_ENV").unwrap_or("development".to_string()),
            // Some Rust shenanigans to set the default value to a boolean false:
            enable_debug_libraries: std::env::var("ENABLE_DEBUG_LIBRARIES")
                .unwrap_or("false".to_string())
                .parse()
                .unwrap_or(false),
        }
    }
}

/// Return the appropriate String denoting the level and breadth of logs depending on the [`TracingConfig`] passed in.
fn build_filter_string(config: &TracingConfig) -> String {
    let crate_name = env!("CARGO_CRATE_NAME");

    match (config.env.as_str(), config.enable_debug_libraries) {
        ("production", true) => "info".to_string(),
        ("production", false) => format!("{crate_name}=info"),

        (_, true) => "trace".to_string(),
        (_, false) => format!("{crate_name}=trace"),
    }
}

/// Build a suitable subscriber based on the context of the environment (i.e production or development). The only difference in subscriber configuration is that in a production context, logs are only sent to `amd.log` and not to `stdout`. This is done on the assumption that when deployed in production, checking terminal logs is neither reliable nor convenient.
///
/// # Arguments
///
/// * env: A string that can be set to "production" in order to disable logging to `stdout` and when set to anything else, enable logging to `stdout`.
/// * filter: The filter used to determine the log level for this subscriber.
///
/// Returns the initialized subscriber inside a [`Box`].
fn build_subscriber<L>(
    env: String,
    filter: L,
) -> anyhow::Result<Box<dyn tracing::Subscriber + Send + Sync>>
where
    L: tracing_subscriber::Layer<tracing_subscriber::Registry> + Send + Sync + 'static,
{
    let file_layer = fmt::layer()
        .pretty()
        .with_ansi(false)
        .with_writer(File::create("amd.log").context("Failed to create log file")?);

    if env != "production" {
        Ok(Box::new(
            tracing_subscriber::registry()
                .with(filter)
                .with(file_layer)
                .with(fmt::layer().pretty().with_writer(std::io::stdout)),
        ))
    } else {
        Ok(Box::new(
            tracing_subscriber::registry().with(filter).with(file_layer),
        ))
    }
}

fn setup_tracing() -> anyhow::Result<ReloadHandle> {
    let config = TracingConfig::load_tracing_config();
    let filter_string = build_filter_string(&config);
    let (filter, reload_handle) = Layer::new(EnvFilter::new(filter_string));

    let boxed_subscriber: Box<dyn tracing::Subscriber + Send + Sync> =
        build_subscriber(config.env, filter)?;
    tracing::subscriber::set_global_default(boxed_subscriber)
        .context("Failed to set subscriber")?;

    Ok(Arc::new(RwLock::new(reload_handle)))
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenv::dotenv().ok();
    let reload_handle = setup_tracing().context("Failed to setup tracing")?;

    info!("Tracing initialized. Continuing main...");
    let mut data = Data {
        reaction_roles: HashMap::new(),
        log_reload_handle: reload_handle,
    };
    populate_data_with_reaction_roles(&mut data);

    let discord_token =
        std::env::var("DISCORD_TOKEN").context("DISCORD_TOKEN was not found in the ENV")?;
    let owner_id: u64 = std::env::var("OWNER_ID")
        .context("OWNER_ID was not found in the ENV")?
        .parse()
        .context("Failed to parse owner_id")?;
    let owner_user_id = UserId::from(owner_id);

    let framework = Framework::builder()
        .options(FrameworkOptions {
            commands: commands::get_commands(),
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler(ctx, event, framework, data))
            },
            prefix_options: PrefixFrameworkOptions {
                prefix: Some(String::from("$")),
                ..Default::default()
            },
            owners: HashSet::from([owner_user_id]),
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                scheduler::run_scheduler(ctx.clone()).await;
                Ok(data)
            })
        })
        .build();

    let mut client = serenity::client::ClientBuilder::new(
        discord_token,
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
