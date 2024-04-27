use eyre::{Result, WrapErr};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};
use tracing_tree::HierarchicalLayer;

mod config;
use config::Config;

mod export;

mod prompt;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    // enable tracing
    Registry::default()
        .with(EnvFilter::from_default_env())
        .with(
            HierarchicalLayer::new(4)
                .with_targets(true)
                .with_indent_lines(true)
                .with_bracketed_fields(true)
                .with_thread_names(false)
                .with_thread_ids(true),
        )
        .init();

    let app_config = envy::from_env::<Config>().wrap_err("Failed to parse config from env")?;
    let mut client = export::make_client(&app_config)
        .await
        .wrap_err("Failed to make client")?;
    export::login(&mut client, &app_config)
        .await
        .wrap_err("Failed to login")?;

    let source_chat = export::find_chat(&mut client, app_config.source_chat_id())
        .await
        .wrap_err("Failed to find source chat")?;

    let me = client.get_me().await.wrap_err("Failed to get me")?.pack();

    export::forward_all(&mut client, &app_config, source_chat, me)
        .await
        .wrap_err("Failed to forward messages")?;

    Ok(())
}
