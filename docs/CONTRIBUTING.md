Thank you for considering contributing to `amD`! This project has no novel guidelines for contributions you need to consider. If you have experience contributing to Open Source Software already, then you can skip right ahead to the parts of the documentation you're interested in. If you're new to Open Source, check out [this helpful guide](https://opensource.guide/how-to-contribute) that'll help you get started.

We don't have any strict rules when it comes to contributing. However, when opening a PR or an issue, writing a commit, or leaving a comment or a review, please remember that verbosity and attention to detail in your communication is appreciated. Try to include as much detail as reasonable in your messages in order to facilitate other developers and prevent guesswork.

The rest of this document will explain the high-level details of the internals of the bot as well it's interactions with external programs.

# Documentation

### Command Handling

This bot uses `poise`, a command framework built on top of `serenity`. You can add commands in the `commands` module and get them registered using the `get_commands` function.

```rust
// Example command in src/commands.rs
#[poise::command(prefix_command)]
async fn amdctl(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("amD is up and running.").await?;
    Ok(())
}

pub fn get_commands() -> Vec<poise::Command<Data, Error>> {
    vec![amdctl()]
}
```

### Reaction Roles

amD supports automatic role assignment based on emoji reactions to specific messages. You can configure which messages and reactions trigger role assignemnt by modifying the `reaction_roles` Hashmap in the bot's `Data` struct in the `initialize_data()` function.

```rust
    reaction_roles: HashMap::new(),

    // Role IDs, use `\@<ROLE>` to get the ID on Discord
    let archive_role_id = RoleId::new(ARCHIVE_ROLE_ID);
    let mobile_role_id = RoleId::new(MOBILE_ROLE_ID);
    let systems_role_id = RoleId::new(SYSTEMS_ROLE_ID);
    ... /* excluded for brevity */

    let message_roles = [
        // Give y role if reacted with x emoji in a hashmap pair (x, y)
        (ReactionType::Unicode("📁".to_string()), archive_role_id),
        (ReactionType::Unicode("📱".to_string()), mobile_role_id),
        (ReactionType::Unicode("⚙️".to_string()), systems_role_id),
        ... /* excluded for brevity */

   ];

    data.reaction_roles.extend::<HashMap<ReactionType, RoleId>>(message_roles.into());

```

The event handler takes care of the rest:

```rust
        // On the event of a reaction being added
        FullEvent::ReactionAdd { add_reaction } => {
            let message_id = MessageId::new(ROLES_MESSAGE_ID);
            // Check if the reaction was added to the message we want and if it is reacted with the
            // emoji we want
            if add_reaction.message_id == message_id && data.reaction_roles.contains_key(&add_reaction.emoji) {
                    // Ensure it is in a server
                    if let Some(guild_id) = add_reaction.guild_id {
                        // Give the member the required role
                        if let Ok(member) =
                            guild_id.member(ctx, add_reaction.user_id.unwrap()).await
                        {
                            if let Err(e) = member.add_role(&ctx.http, data.reaction_roles.get(&add_reaction.emoji).expect("Hard coded value verified earlier.")).await {
                                eprintln!("Error: {:?}", e);
                            }
                        }
                    }
                }
        }
```

### Scheduler

The scheduler system allows you to easily define tasks that should be repeated periodically. Simply define a struct that implements the `task` trait and the `scheduler` module will automatically spawn a thread for your task on startup.

```rust
#[async_trait]
pub trait Task: Send + Sync {
    fn name(&self) -> &'static str;
    fn run_in(&self) -> Duration;
    async fn run(&self, ctx: Context);
}

```
Sample task that runs at 5 am every day:

```rust
pub struct StatusUpdateCheck;

#[async_trait]
impl Task for StatusUpdateCheck {
    fn name(&self) -> &'static str {
        "StatusUpdateCheck"
    }

    fn run_in(&self) -> Duration {
        time_until(5, 0)
    }

    async fn run(&self, ctx: Context) {
    ... /* Excluded for brevity */
    }
```
