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

/// Local-style timestamp (YYYY-MM-DD HH:MM, UTC) for "last synced" and
/// backup folder names. Pure Rust; the sandbox and target systems both
/// lack a guaranteed `date` binary on the PATH we control.
pub fn timestamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64;
    let days = secs.div_euclid(86_400);
    let sod = secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}",
        sod / 3600,
        (sod % 3600) / 60
    )
}

/// Days since the Unix epoch -> (year, month, day), proleptic Gregorian
/// (Howard Hinnant's civil_from_days).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
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

    #[test]
    fn epoch_boundaries() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_782), (2024, 2, 29)); // leap day
        assert_eq!(civil_from_days(20_147), (2025, 2, 28));
    }
}
