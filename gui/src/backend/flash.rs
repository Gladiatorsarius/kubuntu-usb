//! Flash wizard backend: Kubuntu ISO download/cache, post-install
//! settings pre-seed onto the mounted stick.

use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;

/// Kubuntu 24.04 LTS (noble) desktop ISO.
pub const ISO_URL: &str =
    "https://cdimage.ubuntu.com/kubuntu/releases/24.04/release/kubuntu-24.04-desktop-amd64.iso";
pub const ISO_NAME: &str = "kubuntu-24.04-desktop-amd64.iso";

pub fn iso_path() -> PathBuf {
    crate::config::cache_dir().join(ISO_NAME)
}

#[derive(Clone, Debug)]
pub enum DownloadEvent {
    /// Bytes downloaded so far (total unknown - cdimage sends no length on some mirrors).
    Progress(u64),
    Done(PathBuf),
    Failed(String),
}

/// Download the ISO into the cache dir (resuming a partial file), streaming
/// progress. Blocking - call from a worker thread.
pub fn download_iso(tx: Sender<DownloadEvent>) {
    let target = iso_path();
    if let Err(err) = std::fs::create_dir_all(target.parent().unwrap()) {
        let _ = tx.send(DownloadEvent::Failed(err.to_string()));
        return;
    }
    // curl: resume, follow redirects, print progress as "#..." meters we
    // cannot parse reliably - so poll the file size instead.
    let child = Command::new("curl")
        .args(["-fSL", "-C", "-", "-o"])
        .arg(&target)
        .arg(ISO_URL)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    let mut child = match child {
        Ok(child) => child,
        Err(err) => {
            let _ = tx.send(DownloadEvent::Failed(err.to_string()));
            return;
        }
    };
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if status.success() {
                    let _ = tx.send(DownloadEvent::Done(target));
                } else {
                    let _ = tx.send(DownloadEvent::Failed(format!(
                        "curl exited with {status}"
                    )));
                }
                return;
            }
            Ok(None) => {
                if let Ok(meta) = std::fs::metadata(&target) {
                    let _ = tx.send(DownloadEvent::Progress(meta.len()));
                }
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
            Err(err) => {
                let _ = tx.send(DownloadEvent::Failed(err.to_string()));
                return;
            }
        }
    }
}

/// Copy the chosen user's KDE settings onto the mounted stick
/// (step 3 of the wizard). Only the KDE-relevant pieces, matching
/// config/folders.txt in the repo.
pub fn preseed_settings(stick_home: &Path, pc_home: &Path) -> io::Result<()> {
    const KDE_PATHS: &[&str] = &[
        ".config/dolphinrc",
        ".config/kdeglobals",
        ".config/kwinrc",
        ".config/kwinrulesrc",
        ".config/kglobalshortcutsrc",
        ".config/plasma-org.kde.plasma.desktop-appletsrc",
        ".config/plasmashellrc",
        ".config/kscreenlockerrc",
        ".config/kdeconnect",
    ];
    let mut copied = 0;
    for rel in KDE_PATHS {
        let source = pc_home.join(rel);
        if !source.exists() {
            continue;
        }
        let target = stick_home.join(rel);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let status = Command::new("cp")
            .arg("-a")
            .arg(&source)
            .arg(&target)
            .status()?;
        if status.success() {
            copied += 1;
        }
    }
    if copied == 0 {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "no KDE settings found to copy",
        ));
    }
    Ok(())
}
