use eyre::{Result, WrapErr};
use tracing::info;
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
                .with_thread_ids(false),
        )
        .init();

    let app_config = Config::from_env().wrap_err("Failed to load config")?;

    let mut client = export::make_client(&app_config)
        .await
        .wrap_err("Failed to make client")?;
    info!("Client created, logging in...");
    export::login(&mut client, &app_config)
        .await
        .wrap_err("Failed to login")?;
    info!(
        source_chat_id = app_config.source_chat_id(),
        "Logged in, finding source chat..."
    );

    let source_chat = export::find_chat(&mut client, app_config.source_chat_id())
        .await
        .wrap_err("Failed to find source chat")?;
    info!("Found source chat, getting target chat...");

    let target_chat = match app_config.target_chat_id() {
        Some(target_chat_id) => {
            info!(target_chat_id, "Found target chat, getting target chat...");

            export::find_chat(&mut client, target_chat_id)
                .await
                .wrap_err("Failed to find target chat")?
        }
        None => {
            info!("No target chat specified, using my own chat...");

            client.get_me().await.wrap_err("Failed to get me")?.pack()
        }
    };

    export::forward_all(&mut client, &app_config, source_chat, target_chat)
        .await
        .wrap_err("Failed to forward messages")?;
    info!("Export finished");

    Ok(())
}
