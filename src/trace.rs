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

/// Return the appropriate String denoting the level and breadth of logs depending on the [`TracingConfig`] passed in.
fn build_filter_string(debug: bool, enable_debug_libraries: bool) -> String {
    let crate_name = env!("CARGO_CRATE_NAME");

    match (debug, enable_debug_libraries) {
        (true, true) => "info".to_string(),
        (true, false) => format!("{crate_name}=info"),

        (false, true) => "trace".to_string(),
        (false, false) => format!("{crate_name}=trace"),
    }
}

/// Build a suitable subscriber based on the context of the environment (i.e production or development). The only difference in subscriber configuration is that in a production context, logs are only sent to `amd.log` and not to `stdout`. This is done on the assumption that when deployed in production, checking terminal logs is neither reliable nor convenient.
///
/// # Arguments
///
/// * debug: A boolean that can be set to true in order to disable logging to `stdout` and when set to false, enable logging to `stdout`.
/// * filter: The filter used to determine the log level for this subscriber.
///
/// Returns the initialized subscriber inside a [`Box`].
fn build_subscriber<L>(
    debug: bool,
    filter: L,
) -> anyhow::Result<Box<dyn tracing::Subscriber + Send + Sync>>
where
    L: tracing_subscriber::Layer<tracing_subscriber::Registry> + Send + Sync + 'static,
{
    let file_layer = fmt::layer()
        .pretty()
        .with_ansi(false)
        .with_writer(File::create("amd.log").context("Failed to create log file")?);

    if debug {
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

pub fn setup_tracing(debug: bool, enable_debug_libraries: bool) -> anyhow::Result<ReloadHandle> {
    let filter_string = build_filter_string(debug, enable_debug_libraries);
    let (filter, reload_handle) = Layer::new(EnvFilter::new(filter_string));

    let boxed_subscriber: Box<dyn tracing::Subscriber + Send + Sync> =
        build_subscriber(debug, filter)?;
    tracing::subscriber::set_global_default(boxed_subscriber)
        .context("Failed to set subscriber")?;

    Ok(Arc::new(RwLock::new(reload_handle)))
}
