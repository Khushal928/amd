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
//! Module for the set_log_level command.

use crate::{Context, Error};
use anyhow::Context as _;
use tracing::info;
use tracing::instrument;
use tracing_subscriber::EnvFilter;
/// Returns whether the provided `level` String is a valid filter level for tracing.
fn validate_level(level: &String) -> bool {
    const VALID_LEVELS: [&str; 5] = ["trace", "debug", "info", "warn", "error"];
    if !VALID_LEVELS.contains(&level.as_str()) {
        true
    } else {
        false
    }
}

fn build_filter_string(level: String, enable_debug_libraries: bool) -> anyhow::Result<String> {
    let crate_name = env!("CARGO_CRATE_NAME");

    if enable_debug_libraries {
        Ok(level)
    } else {
        Ok(format!("{crate_name}={level}"))
    }
}

#[poise::command(prefix_command, owners_only)]
#[instrument(level = "debug", skip(ctx))]
pub async fn set_log_level(
    ctx: Context<'_>,
    level: String,
    enable_debug_libraries: Option<bool>,
) -> Result<(), Error> {
    if !validate_level(&level) {
        ctx.say("Invalid log level! Use: trace, debug, info, warn, error")
            .await?;
        return Ok(());
    }

    let new_filter_level = build_filter_string(level, enable_debug_libraries.unwrap_or_default())?;

    let data = ctx.data();
    let reload_handle = data.log_reload_handle.write().await;

    if reload_handle
        .reload(EnvFilter::new(&new_filter_level))
        .is_ok()
    {
        ctx.say(format!("Log level changed to **{new_filter_level}**"))
            .await?;
        info!("Log level changed to {}", new_filter_level);
    } else {
        ctx.say("Failed to update log level.").await?;
    }

    Ok(())
}
