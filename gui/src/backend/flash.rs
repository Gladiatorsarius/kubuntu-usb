//! Flash wizard backend: Kubuntu ISO download/cache (with SHA256
//! verification), post-install settings pre-seed onto the mounted stick.

use sha2::{Digest, Sha256};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;

/// Kubuntu 24.04 LTS (noble) desktop ISO - latest point release in the
/// 24.04 series (verified 200 on cdimage.ubuntu.com; the un-suffixed
/// kubuntu-24.04-desktop-amd64.iso name 404s).
pub const ISO_URL: &str =
    "https://cdimage.ubuntu.com/kubuntu/releases/24.04/release/kubuntu-24.04.5-desktop-amd64.iso";
pub const ISO_NAME: &str = "kubuntu-24.04.5-desktop-amd64.iso";
/// Ubuntu publishes the checksums for every image in the release dir.
pub const SHA256SUMS_URL: &str =
    "https://cdimage.ubuntu.com/kubuntu/releases/24.04/release/SHA256SUMS";

pub fn iso_path() -> PathBuf {
    crate::config::cache_dir().join(ISO_NAME)
}

fn iso_part_path() -> PathBuf {
    crate::config::cache_dir().join(format!("{ISO_NAME}.part"))
}

#[derive(Clone, Debug)]
pub enum DownloadEvent {
    /// Bytes downloaded so far (total unknown - cdimage sends no length on some mirrors).
    Progress(u64),
    /// Download complete, SHA256 verification running.
    Verifying,
    /// Verified ISO ready at this path.
    Done(PathBuf),
    Failed(String),
}

/// Download the ISO into the cache dir, verify it against Ubuntu's
/// published SHA256SUMS, then rename it into place. The partial download
/// lives in <name>.part until it is verified, so a half-finished file
/// never shows as "already downloaded". Blocking - call from a worker
/// thread.
pub fn download_iso(tx: Sender<DownloadEvent>) {
    let part = iso_part_path();
    let target = iso_path();
    if let Err(err) = std::fs::create_dir_all(part.parent().unwrap()) {
        let _ = tx.send(DownloadEvent::Failed(err.to_string()));
        return;
    }
    // curl: resume, follow redirects. Progress is polled from the file
    // size instead of parsing curl's meter.
    let child = Command::new("curl")
        .args(["-fSL", "-C", "-", "-o"])
        .arg(&part)
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
                if !status.success() {
                    let _ = tx.send(DownloadEvent::Failed(format!(
                        "curl exited with {status}"
                    )));
                    return;
                }
                break;
            }
            Ok(None) => {
                if let Ok(meta) = std::fs::metadata(&part) {
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

    let _ = tx.send(DownloadEvent::Verifying);
    match verify_iso(&part) {
        Ok(()) => {
            if let Err(err) = std::fs::rename(&part, &target) {
                let _ = tx.send(DownloadEvent::Failed(format!(
                    "could not move ISO into place: {err}"
                )));
                return;
            }
            let _ = tx.send(DownloadEvent::Done(target));
        }
        Err(err) => {
            // Never keep an unverified image around.
            let _ = std::fs::remove_file(&part);
            let _ = tx.send(DownloadEvent::Failed(format!(
                "ISO verification failed: {err}"
            )));
        }
    }
}

/// Fetch the expected SHA256 for ISO_NAME from Ubuntu's SHA256SUMS.
fn expected_sha256() -> io::Result<String> {
    let out = Command::new("curl")
        .args(["-fSL", SHA256SUMS_URL])
        .output()?;
    if !out.status.success() {
        return Err(io::Error::other(format!(
            "could not fetch SHA256SUMS (curl exited with {})",
            out.status
        )));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    parse_sha256sums(&text, ISO_NAME).ok_or_else(|| {
        io::Error::other(format!("no checksum for {ISO_NAME} in SHA256SUMS"))
    })
}

/// Parse one "<hex> *<name>" line out of a SHA256SUMS file.
fn parse_sha256sums(text: &str, name: &str) -> Option<String> {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let hash = parts.next()?;
            let file = parts.next()?;
            Some((hash, file.trim_start_matches('*')))
        })
        .find(|(_, file)| *file == name)
        .map(|(hash, _)| hash.to_lowercase())
}

/// SHA256 the downloaded image and compare with Ubuntu's checksum.
fn verify_iso(part: &Path) -> io::Result<()> {
    let expected = expected_sha256()?;
    let mut file = std::fs::File::open(part)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    let actual = format!("{:x}", hasher.finalize());
    if actual == expected {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "checksum mismatch (expected {expected}, got {actual})"
        )))
    }
}

/// Copy the chosen user's KDE settings onto the mounted stick
/// (step 3 of the wizard). Only the KDE-relevant pieces, matching
/// config/folders.txt in the repo. Fails loudly: the error names every
/// path that could not be copied.
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
    let mut failed: Vec<String> = Vec::new();
    for rel in KDE_PATHS {
        let source = pc_home.join(rel);
        if !source.exists() {
            continue;
        }
        let target = stick_home.join(rel);
        let result = (|| -> io::Result<std::process::ExitStatus> {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            Command::new("cp").arg("-a").arg(&source).arg(&target).status()
        })();
        match result {
            Ok(status) if status.success() => copied += 1,
            Ok(status) => failed.push(format!("{rel} (cp exited with {status})")),
            Err(err) => failed.push(format!("{rel} ({err})")),
        }
    }
    if !failed.is_empty() {
        return Err(io::Error::other(format!(
            "could not copy: {}",
            failed.join(", ")
        )));
    }
    if copied == 0 {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "no KDE settings found to copy",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sha256sums_line() {
        let sums = "abc123 *kubuntu-24.04.5-desktop-amd64.iso\ndef456 *kubuntu-24.04.5-desktop-arm64.iso\n";
        assert_eq!(
            parse_sha256sums(sums, "kubuntu-24.04.5-desktop-amd64.iso"),
            Some("abc123".to_string())
        );
        assert_eq!(parse_sha256sums(sums, "kubuntu-24.04.5-desktop-i386.iso"), None);
    }
}
