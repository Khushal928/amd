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
use anyhow::Context as _;
use serenity::all::RoleId;
use tracing::{info, trace};
use tracing_subscriber::EnvFilter;

use crate::{
    ids::{FOURTH_YEAR_ROLE_ID, THIRD_YEAR_ROLE_ID},
    Context, Data, Error,
};

/// Checks if the author has the Fourth Year or Third Year role. Can be used as an authorization procedure for other commands.
async fn is_privileged(ctx: &Context<'_>) -> bool {
    if let Some(guild_id) = ctx.guild_id() {
        if let Ok(member) = guild_id.member(ctx, ctx.author().id).await {
            return member.roles.contains(&RoleId::new(FOURTH_YEAR_ROLE_ID))
                || member.roles.contains(&RoleId::new(THIRD_YEAR_ROLE_ID));
        }
    }

    false
}

#[poise::command(prefix_command)]
async fn amdctl(ctx: Context<'_>) -> Result<(), Error> {
    trace!("Running amdctl command");
    ctx.say("amD is up and running.").await?;
    Ok(())
}

#[poise::command(prefix_command, owners_only)]
async fn set_log_level(ctx: Context<'_>, level: String) -> Result<(), Error> {
    trace!("Running set_log_level command");
    let data = ctx.data();
    let reload_handle = data.log_reload_handle.write().await;

    let enable_debug_libraries_string = std::env::var("ENABLE_DEBUG_LIBRARIES")
        .context("ENABLE_DEBUG_LIBRARIES was not found in the ENV")?;
    let enable_debug_libraries: bool = enable_debug_libraries_string
        .parse()
        .context("Failed to parse ENABLE_DEBUG_LIBRARIES")?;
    let crate_name = env!("CARGO_CRATE_NAME");
    let new_filter = match level.to_lowercase().as_str() {
        "trace" => {
            if enable_debug_libraries {
                "trace".to_string()
            } else {
                format!("{crate_name}=trace")
            }
        }
        "debug" => {
            if enable_debug_libraries {
                "debug".to_string()
            } else {
                format!("{crate_name}=debug")
            }
        }
        "info" => {
            if enable_debug_libraries {
                "info".to_string()
            } else {
                format!("{crate_name}=info")
            }
        }
        "warn" => {
            if enable_debug_libraries {
                "warn".to_string()
            } else {
                format!("{crate_name}=warn")
            }
        }
        "error" => {
            if enable_debug_libraries {
                "error".to_string()
            } else {
                format!("{crate_name}=error")
            }
        }
        _ => {
            ctx.say("Invalid log level! Use: trace, debug, info, warn, error")
                .await?;
            return Ok(());
        }
    };

    if reload_handle.reload(EnvFilter::new(&new_filter)).is_ok() {
        ctx.say(format!("Log level changed to **{new_filter}**"))
            .await?;
        info!("Log level changed to {}", new_filter);
    } else {
        ctx.say("Failed to update log level.").await?;
    }

    Ok(())
}

/// Returns a vector containg [Poise Commands][`poise::Command`]
pub fn get_commands() -> Vec<poise::Command<Data, Error>> {
    vec![amdctl(), set_log_level()]
}
