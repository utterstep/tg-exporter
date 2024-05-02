use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use eyre::WrapErr;
use secrecy::SecretString;
use serde::Deserialize;

const API_ID: i32 = 20625378;

fn default_api_id() -> i32 {
    API_ID
}

fn default_session_path() -> PathBuf {
    "exporter.session".into()
}

fn default_media_path() -> PathBuf {
    "media".into()
}

fn default_sleep_duration() -> u64 {
    2
}

#[derive(Deserialize)]
pub struct ConfigInner {
    #[serde(default = "default_api_id")]
    api_id: i32,
    api_hash: SecretString,
    source_chat_id: i64,
    export_hashtags: String,
    #[serde(default = "default_session_path")]
    session_path: PathBuf,
    #[serde(default = "default_media_path")]
    media_path: PathBuf,
    #[serde(default = "default_sleep_duration")]
    sleep_duration: u64,
}

impl ConfigInner {
    pub fn api_id(&self) -> i32 {
        self.api_id
    }

    pub fn api_hash(&self) -> &SecretString {
        &self.api_hash
    }

    pub fn source_chat_id(&self) -> i64 {
        self.source_chat_id
    }

    pub fn export_hashtags(&self) -> &str {
        &self.export_hashtags
    }

    pub fn session_path(&self) -> &Path {
        &self.session_path
    }

    pub fn media_path(&self) -> &Path {
        &self.media_path
    }

    pub fn sleep_duration(&self) -> u64 {
        self.sleep_duration
    }
}

pub struct Config {
    inner: Arc<ConfigInner>,
}

impl Config {
    pub fn from_env() -> eyre::Result<Self> {
        let config =
            envy::from_env::<ConfigInner>().wrap_err("Failed to parse config from environment")?;
        Ok(Self {
            inner: Arc::new(config),
        })
    }
}

impl std::ops::Deref for Config {
    type Target = ConfigInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
