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
//! This module is responsible for configuring and initializing `tracing`.
use std::{fs::File, sync::Arc};
use tokio::sync::RwLock;

use anyhow::Context;
use tracing_subscriber::{
    fmt,
    layer::SubscriberExt,
    reload::{self, Layer},
    EnvFilter, Registry,
};

pub type ReloadHandle = Arc<RwLock<reload::Handle<EnvFilter, Registry>>>;

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

pub fn setup_tracing() -> anyhow::Result<ReloadHandle> {
    let config = TracingConfig::load_tracing_config();
    let filter_string = build_filter_string(&config);
    let (filter, reload_handle) = Layer::new(EnvFilter::new(filter_string));

    let boxed_subscriber: Box<dyn tracing::Subscriber + Send + Sync> =
        build_subscriber(config.env, filter)?;
    tracing::subscriber::set_global_default(boxed_subscriber)
        .context("Failed to set subscriber")?;

    Ok(Arc::new(RwLock::new(reload_handle)))
}
