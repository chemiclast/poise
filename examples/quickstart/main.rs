use poise::serenity_prelude as serenity;

struct Data {} // User data, which is stored and accessible in all command invocations
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

/// Displays your or another user's account creation date
#[poise::command(slash_command, prefix_command)]
async fn age(
    ctx: Context<'_>,
    #[description = "Selected user"] user: Option<serenity::User>,
) -> Result<(), Error> {
    let u = user.as_ref().unwrap_or_else(|| ctx.author());
    let response = format!("{}'s account was created at {}", u.name, u.created_at());
    ctx.say(response).await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let intents = serenity::GatewayIntents::non_privileged();
    let framework = poise::Framework::new(
        poise::FrameworkOptions {
            commands: vec![age(), register()],
            ..Default::default()
        },
        move |_ctx, _ready, _framework| Box::pin(async move { Ok(Data {}) }),
    );
    let mut client = serenity::Client::builder(token, intents)
        .framework(framework)
        .await
        .unwrap();
    client.start().await.unwrap();
}
