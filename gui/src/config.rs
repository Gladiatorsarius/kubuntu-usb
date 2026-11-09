//! Persistent settings, stored as TOML in
//! `~/.config/kubuntu-usb-sync/config.toml`.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SyncMode {
    /// Two-way sync (Unison, conflicts flagged).
    BothWays,
    /// One-way: the PC copy wins (Unison -force <pc root>).
    PcWins,
    /// One-way: the stick copy wins (Unison -force <stick root>).
    StickWins,
}

impl SyncMode {
    #[allow(dead_code)] // used by the sync backend
    pub fn label(self) -> &'static str {
        match self {
            SyncMode::BothWays => "↔ Both ways",
            SyncMode::PcWins => "PC wins",
            SyncMode::StickWins => "Stick wins",
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FolderConfig {
    pub path: String,
    #[serde(default)]
    pub description: String,
    pub mode: SyncMode,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub folders: Vec<FolderConfig>,
    pub auto_detect: bool,
    pub auto_sync: bool,
    pub copy_user: String,
    /// Apps ticked on the Apps screen (apt package names or flatpak IDs).
    pub selected_apps: Vec<String>,
    /// Filesystem label of the stick's root partition. Coupled to the udev
    /// rule (packaging/etc/udev/rules.d/99-kubuntu-usb-sync.rules matches
    /// ENV{ID_FS_LABEL}=="KUBUNTU-USB"): change it in both places or the
    /// auto-sync trigger stops firing.
    pub stick_label: String,
    /// Free-form timestamp of the last successful sync, shown on the Sync screen.
    pub last_synced: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            folders: vec![
                FolderConfig {
                    path: "~/School".into(),
                    description: "Notes, homework and presentations".into(),
                    mode: SyncMode::BothWays,
                },
                FolderConfig {
                    path: "~/.config".into(),
                    description: "Settings".into(),
                    mode: SyncMode::PcWins,
                },
                FolderConfig {
                    path: "~/Projects".into(),
                    description: "Code and assets".into(),
                    mode: SyncMode::BothWays,
                },
            ],
            auto_detect: true,
            auto_sync: false,
            copy_user: "gladi".into(),
            selected_apps: vec![
                "Firefox".into(),
                "Kate".into(),
                "htop".into(),
                "VLC".into(),
            ],
            stick_label: "KUBUNTU-USB".into(),
            last_synced: None,
        }
    }
}

pub fn config_dir() -> PathBuf {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    home.join(".config").join("kubuntu-usb-sync")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

#[allow(dead_code)] // used by the flash wizard (ISO cache)
pub fn cache_dir() -> PathBuf {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    home.join(".cache").join("kubuntu-usb-sync")
}

impl Settings {
    pub fn load() -> Self {
        let path = config_path();
        match fs::read_to_string(&path) {
            Ok(text) => toml::from_str(&text).unwrap_or_else(|err| {
                eprintln!("config: could not parse {}: {err}", path.display());
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        fs::create_dir_all(config_dir())?;
        let text = toml::to_string_pretty(self).map_err(std::io::Error::other)?;
        let path = config_path();
        // Keep the previous config as config.toml.bak so a bad write never
        // destroys the user's only copy.
        if path.exists() {
            let _ = fs::copy(&path, path.with_extension("toml.bak"));
        }
        fs::write(path, text)
    }
}

/// Local timestamp (YYYY-MM-DD HH:MM) for "last synced" and backup
/// folder names. chrono resolves the local offset with its own TZif
/// parsing (no libc localtime_r, so this is sound from any thread).
pub fn timestamp() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M").to_string()
}

#[cfg(test)]
mod timestamp_tests {
    use super::*;

    #[test]
    fn timestamp_shape() {
        let ts = timestamp();
        assert_eq!(ts.len(), 16, "got {ts:?}");
        assert_eq!(&ts[4..5], "-");
        assert_eq!(&ts[7..8], "-");
        assert_eq!(&ts[10..11], " ");
        assert_eq!(&ts[13..14], ":");
    }
}
