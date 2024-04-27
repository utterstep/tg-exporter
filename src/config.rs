use std::path::{Path, PathBuf};

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

#[derive(Deserialize)]
pub struct Config {
    #[serde(default = "default_api_id")]
    api_id: i32,
    api_hash: SecretString,
    source_chat_id: i64,
    export_hashtags: String,
    #[serde(default = "default_session_path")]
    session_path: PathBuf,
    #[serde(default = "default_media_path")]
    media_path: PathBuf,
}

impl Config {
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
}
