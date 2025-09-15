use anyhow::anyhow;
use anyhow::Context as _;
use tracing::debug;

use crate::graphql::models::Streak;

use super::models::Member;

pub async fn increment_streak(member: &mut Member) -> anyhow::Result<()> {
    let request_url = std::env::var("ROOT_URL").context("ROOT_URL was not found in ENV")?;

    let client = reqwest::Client::new();
    let mutation = format!(
        r#"
        mutation {{
            incrementStreak(input: {{ memberId: {} }}) {{
                currentStreak
                maxStreak
            }}
        }}"#,
        member.member_id
    );

    debug!("Sending mutation {}", mutation);
    let response = client
        .post(request_url)
        .json(&serde_json::json!({"query": mutation}))
        .send()
        .await
        .context("Failed to succesfully post query to Root")?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Server responded with an error: {:?}",
            response.status()
        ));
    }
    let response_json: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse response JSON")?;
    debug!("Response: {}", response_json);

    if let Some(data) = response_json
        .get("data")
        .and_then(|data| data.get("incrementStreak"))
    {
        let current_streak =
            data.get("currentStreak")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| anyhow!("current_streak was parsed as None"))? as i32;
        let max_streak =
            data.get("maxStreak")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| anyhow!("max_streak was parsed as None"))? as i32;

        if member.streak.is_empty() {
            member.streak.push(Streak {
                current_streak,
                max_streak,
            });
        } else {
            for streak in &mut member.streak {
                streak.current_streak = current_streak;
                streak.max_streak = max_streak;
            }
        }
    } else {
        return Err(anyhow!(
            "Failed to access data from response: {}",
            response_json
        ));
    }

    Ok(())
}

pub async fn reset_streak(member: &mut Member) -> anyhow::Result<()> {
    let request_url = std::env::var("ROOT_URL").context("ROOT_URL was not found in the ENV")?;

    let client = reqwest::Client::new();
    let mutation = format!(
        r#"
        mutation {{
            resetStreak(input: {{ memberId: {} }}) {{
                currentStreak
                maxStreak
            }}
        }}"#,
        member.member_id
    );

    debug!("Sending mutation {}", mutation);
    let response = client
        .post(&request_url)
        .json(&serde_json::json!({ "query": mutation }))
        .send()
        .await
        .context("Failed to succesfully post query to Root")?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Server responded with an error: {:?}",
            response.status()
        ));
    }

    let response_json: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse response JSON")?;
    debug!("Response: {}", response_json);

    if let Some(data) = response_json
        .get("data")
        .and_then(|data| data.get("resetStreak"))
    {
        let current_streak =
            data.get("currentStreak")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| anyhow!("current_streak was parsed as None"))? as i32;
        let max_streak =
            data.get("maxStreak")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| anyhow!("max_streak was parsed as None"))? as i32;

        if member.streak.is_empty() {
            member.streak.push(Streak {
                current_streak,
                max_streak,
            });
        } else {
            for streak in &mut member.streak {
                streak.current_streak = current_streak;
                streak.max_streak = max_streak;
            }
        }
    } else {
        return Err(anyhow!("Failed to access data from {}", response_json));
    }

    Ok(())
}
