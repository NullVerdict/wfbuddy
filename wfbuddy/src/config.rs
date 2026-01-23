//! Persistent application configuration.
//!
//! Stored as JSON in a platform-appropriate config directory.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// On-disk configuration for the application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Target window application name (from `xcap::Window::app_name()`).
    ///
    /// This is reasonably stable across restarts. If multiple windows share the
    /// same app name, the first match is used.
    pub app_name: String,

    /// Poll interval (seconds) for lightweight screen checks.
    pub poll_delay_s: f32,

    /// UI theme colors sampled from the in-game options screen.
    pub theme: ie::Theme,

    /// Optional max capture height (downscales large captures for performance).
    pub max_capture_height: Option<u32>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            app_name: "steam_app_230410".to_string(),
            poll_delay_s: 1.0,
            theme: ie::Theme::WHITE,
            max_capture_height: Some(1080),
        }
    }
}

impl Config {
    /// Path to the config file.
    pub fn path() -> Result<PathBuf> {
        let base = dirs::config_dir().context("config_dir() unavailable")?;
        Ok(base.join("wfbuddy.json"))
    }

    /// Load configuration from disk, falling back to defaults on missing file.
    pub fn load_or_default() -> Self {
        match Self::try_load() {
            Ok(cfg) => cfg,
            Err(err) => {
                tracing::warn!(error = %err, "failed to load config; using defaults");
                Self::default()
            }
        }
    }

    /// Try to load configuration from disk.
    pub fn try_load() -> Result<Self> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let json = fs::read_to_string(&path).with_context(|| format!("read {:?}", path))?;
        let cfg = serde_json::from_str(&json).with_context(|| format!("parse {:?}", path))?;
        Ok(cfg)
    }

    /// Save configuration to disk.
    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("create {:?}", parent))?;
        }
        let json = serde_json::to_string_pretty(self).context("serialize config")?;
        fs::write(&path, json).with_context(|| format!("write {:?}", path))?;
        Ok(())
    }
}
