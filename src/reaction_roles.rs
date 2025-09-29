use std::collections::HashMap;

use serenity::all::{Context as SerenityContext, MessageId, Reaction, ReactionType, RoleId};
use tracing::debug;

use crate::{
    ids::{
        AI_ROLE_ID, ARCHIVE_ROLE_ID, DEVOPS_ROLE_ID, MOBILE_ROLE_ID, RESEARCH_ROLE_ID,
        ROLES_MESSAGE_ID, SYSTEMS_ROLE_ID, WEB_ROLE_ID,
    },
    Data, Error,
};

impl Data {
    pub fn populate_with_reaction_roles(&mut self) {
        let roles = [
            (
                ReactionType::Unicode("📁".to_string()),
                RoleId::new(ARCHIVE_ROLE_ID),
            ),
            (
                ReactionType::Unicode("📱".to_string()),
                RoleId::new(MOBILE_ROLE_ID),
            ),
            (
                ReactionType::Unicode("⚙️".to_string()),
                RoleId::new(SYSTEMS_ROLE_ID),
            ),
            (
                ReactionType::Unicode("🤖".to_string()),
                RoleId::new(AI_ROLE_ID),
            ),
            (
                ReactionType::Unicode("📜".to_string()),
                RoleId::new(RESEARCH_ROLE_ID),
            ),
            (
                ReactionType::Unicode("🚀".to_string()),
                RoleId::new(DEVOPS_ROLE_ID),
            ),
            (
                ReactionType::Unicode("🌐".to_string()),
                RoleId::new(WEB_ROLE_ID),
            ),
        ];

        self.reaction_roles
            .extend::<HashMap<ReactionType, RoleId>>(roles.into());
    }
}

pub async fn handle_reaction(
    ctx: &SerenityContext,
    reaction: &Reaction,
    data: &Data,
    is_add: bool,
) -> Result<(), Error> {
    if !is_relevant_reaction(reaction.message_id, &reaction.emoji, data) {
        return Ok(());
    }

    debug!("Handling {:?} from {:?}.", reaction.emoji, reaction.user_id);

    let guild_id = reaction.guild_id.ok_or("No guild_id")?;
    let user_id = reaction.user_id.ok_or("No user_id")?;
    let member = guild_id.member(ctx, user_id).await?;
    let role_id = data
        .reaction_roles
        .get(&reaction.emoji)
        .ok_or("No role mapping")?;

    if is_add {
        member.add_role(&ctx.http, *role_id).await?;
    } else {
        member.remove_role(&ctx.http, *role_id).await?;
    }

    Ok(())
}

fn is_relevant_reaction(message_id: MessageId, emoji: &ReactionType, data: &Data) -> bool {
    message_id == MessageId::new(ROLES_MESSAGE_ID) && data.reaction_roles.contains_key(emoji)
}
