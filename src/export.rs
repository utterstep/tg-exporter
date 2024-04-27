use std::path::Path;

use eyre::{Result, WrapErr};
use grammers_client::types::{Downloadable, Media, Message};
use grammers_client::{Client, Config as ClientConfig, InputMessage, SignInError};
use grammers_session::{PackedChat, Session};
use mime::Mime;
use mime_guess::mime;
use secrecy::{ExposeSecret, SecretString};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info, warn};

use crate::prompt::{prompt, prompt_secret};
use crate::Config;

#[tracing::instrument(skip_all, err)]
pub async fn make_client(app_config: &Config) -> Result<Client> {
    let telegram_config = ClientConfig {
        session: Session::load_file_or_create(app_config.session_path())
            .wrap_err("Failed to load session")?,
        api_id: app_config.api_id(),
        api_hash: app_config.api_hash().expose_secret().clone(),
        params: Default::default(),
    };

    Client::connect(telegram_config)
        .await
        .wrap_err("Failed to connect to Telegram")
}

#[tracing::instrument(skip_all, err)]
pub async fn login(client: &mut Client, app_config: &Config) -> Result<()> {
    if client
        .is_authorized()
        .await
        .wrap_err("Failed to check if authorized")?
    {
        info!("Already authorized");
        return Ok(());
    }

    let phone_number =
        prompt("Enter your phone number: ").wrap_err("Failed to read phone number")?;
    let token = client.request_login_code(phone_number.trim()).await?;
    let code: SecretString =
        prompt_secret("Enter the code you received: ").wrap_err("Failed to read code")?;
    let signed_in = client.sign_in(&token, code.expose_secret().trim()).await;

    match signed_in {
        Err(SignInError::PasswordRequired(password_token)) => {
            let hint = password_token.hint().unwrap_or("None");
            let mut n_tries = 0;

            while n_tries < 3 {
                let tries_left = 3 - n_tries;
                let prompt_message =
                    format!("[{tries_left} / 3] Enter the password (hint {}): ", &hint);
                let password =
                    prompt_secret(prompt_message.as_str()).wrap_err("Failed to read password")?;

                match client
                    .check_password(password_token.clone(), password.expose_secret().trim())
                    .await
                    .wrap_err("Failed to check password")
                {
                    Ok(_) => break,
                    Err(e) => {
                        warn!("Failed to check password: {}", e);

                        n_tries += 1;
                        if n_tries == 3 {
                            return Err(e).wrap_err("Failed to sign in");
                        }
                    }
                }
            }
        }
        Ok(_) => (),
        Err(e) => return Err(e).wrap_err("Failed to sign in"),
    };

    info!("Signed in");

    match client.session().save_to_file(app_config.session_path()) {
        Ok(_) => {
            info!("Session saved");
            Ok(())
        }
        Err(e) => {
            client.sign_out_disconnect().await?;

            return Err(e).wrap_err("Failed to save session");
        }
    }
}

#[tracing::instrument(skip(client), err)]
pub async fn find_chat(client: &mut Client, chat_id: i64) -> Result<PackedChat> {
    let mut chats = client.iter_dialogs();

    while let Some(dialog) = chats.next().await.wrap_err("Failed to get next dialog")? {
        let chat = dialog.chat();
        debug!(chat_id = chat.id(), chat_title = ?chat.name(), "Processing chat");
        if chat.id() == chat_id {
            return Ok(chat.pack());
        }
    }

    Err(eyre::eyre!("Failed to find chat"))
}

#[tracing::instrument(skip_all, fields(source_chat_id = source_chat.id, target_chat_id = target_chat.id), err)]
pub async fn forward_all(
    client: &mut Client,
    app_config: &Config,
    source_chat: PackedChat,
    target_chat: PackedChat,
) -> Result<()> {
    let mut messages = client.iter_messages(source_chat);

    while let Some(message) = messages
        .next()
        .await
        .wrap_err("Failed to get next message")?
    {
        forward_message(client, app_config, target_chat, message)
            .await
            .wrap_err("Failed to forward message")?;

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    Ok(())
}

#[tracing::instrument(skip_all, fields(message_id = message.id()), err)]
pub async fn forward_message(
    client: &mut Client,
    app_config: &Config,
    target_chat: PackedChat,
    message: Message,
) -> Result<()> {
    if check_if_forwarded(client, target_chat, &message).await? {
        info!("Message already forwarded");
        return Ok(());
    }

    let hashtags = app_config.export_hashtags();
    let grouped = message.grouped_id();

    info!("Processing message");
    let dedup_hash = format!("#{dedup_tag}", dedup_tag = message_dedup_tag(&message));

    let text = format!(
        "{dedup_hash} {hashtags}{grouped}\n\n{message}",
        dedup_hash = dedup_hash,
        hashtags = hashtags,
        grouped = match grouped {
            Some(group_id) => format!(" group {}", group_id),
            None => "".to_string(),
        },
        message = message.text()
    );

    let mut forwarded = InputMessage::text(text);

    let media = message.media();

    if let Some(media) = media {
        info!("Downloading media");

        let dest = format!(
            "{}/message-{}{}",
            app_config.media_path().to_string_lossy(),
            &message.id().to_string(),
            get_file_extension(&media)
        );

        // create media directory if it doesn't exist
        std::fs::create_dir_all(app_config.media_path())
            .wrap_err("Failed to create media directory")?;

        let downloadable = Downloadable::Media(media);
        let mut iter = client.iter_download(&downloadable);

        while let Some(chunk) = iter.next().await.wrap_err("Failed to get next chunk")? {
            debug!(chunk_size = chunk.len(), "Writing chunk");

            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&dest)
                .await
                .wrap_err("Failed to open media file")?;

            file.write_all(&chunk)
                .await
                .wrap_err("Failed to write to media file")?;
        }

        message
            .download_media(&Path::new(dest.as_str()))
            .await
            .expect("Error downloading message");

        info!("Media downloaded, reuploading");
        let media_input = client.upload_file(&Path::new(dest.as_str())).await?;
        info!("Media reuploaded");

        forwarded = forwarded.file(media_input);
    }

    client
        .send_message(target_chat, forwarded)
        .await
        .wrap_err("Failed to send forwarded message")?;

    info!("Message forwarded");
    Ok(())
}

#[tracing::instrument(skip(client, target_chat, message), fields(message_id = message.id()), err)]
pub async fn check_if_forwarded(
    client: &mut Client,
    target_chat: PackedChat,
    message: &Message,
) -> Result<bool> {
    let hashtag_to_search = format!("#{dedup_tag}", dedup_tag = message_dedup_tag(message));
    debug!(hashtag = ?hashtag_to_search, "Searching for message");

    let mut results = client
        .search_messages(target_chat)
        .query(&hashtag_to_search);

    while let Some(message) = results
        .next()
        .await
        .wrap_err("Failed to search for message")?
    {
        if message.text().contains(&hashtag_to_search) {
            return Ok(true);
        }
    }

    Ok(false)
}

fn get_file_extension(media: &Media) -> String {
    match media {
        Media::Photo(_) => ".jpg".to_string(),
        Media::Sticker(sticker) => get_mime_extension(sticker.document.mime_type()),
        Media::Document(document) => {
            let name = document.name();

            if name.is_empty() {
                get_mime_extension(document.mime_type())
            } else {
                let ext = std::path::Path::new(name).extension().unwrap_or_default();
                format!(".{}", ext.to_string_lossy())
            }
        }
        Media::Contact(_) => ".vcf".to_string(),
        _ => String::new(),
    }
}

fn get_mime_extension(mime_type: Option<&str>) -> String {
    mime_type
        .map(|m| {
            let mime: Mime = m.parse().unwrap();
            format!(".{}", mime.subtype())
        })
        .unwrap_or_default()
}

fn message_dedup_tag(message: &Message) -> String {
    format!(
        "{chat_id}_{message_id}",
        chat_id = message.chat().id(),
        message_id = message.id()
    )
}
