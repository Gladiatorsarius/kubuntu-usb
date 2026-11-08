//! USB drive detection via `lsblk --json`.
//!
//! Mirrors the safety rule from vm/boot-stick.sh: only devices whose
//! transport is USB are ever offered for flashing or raw VM attach.

use serde::Deserialize;
use std::io;
use std::process::Command;

#[derive(Clone, Debug)]
pub struct Partition {
    pub path: String,
    pub label: Option<String>,
    pub fstype: Option<String>,
    pub mountpoint: Option<String>,
}

#[derive(Clone, Debug)]
pub struct UsbDrive {
    /// Whole-disk device, e.g. /dev/sdb.
    pub device: String,
    pub model: String,
    pub size_bytes: u64,
    pub partitions: Vec<Partition>,
    /// True while any partition of the drive is mounted.
    pub mounted: bool,
}

impl UsbDrive {
    pub fn size_gib(&self) -> f64 {
        self.size_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }

    pub fn display_name(&self) -> String {
        format!("{} ({:.0} GB)", self.model, self.size_gib())
    }

    /// The partition carrying the given filesystem label, if any.
    pub fn find_label(&self, label: &str) -> Option<&Partition> {
        self.partitions
            .iter()
            .find(|part| part.label.as_deref() == Some(label))
    }
}

#[derive(Deserialize)]
struct Lsblk {
    blockdevices: Vec<Device>,
}

#[derive(Deserialize)]
struct Device {
    path: Option<String>,
    size: Option<u64>,
    model: Option<String>,
    tran: Option<String>,
    label: Option<String>,
    fstype: Option<String>,
    mountpoints: Option<Vec<Option<String>>>,
    children: Option<Vec<Device>>,
}

pub fn list_usb_drives() -> io::Result<Vec<UsbDrive>> {
    let out = Command::new("lsblk")
        .args([
            "--json",
            "--bytes",
            "-o",
            "PATH,SIZE,MODEL,TRAN,LABEL,FSTYPE,MOUNTPOINTS",
        ])
        .output()?;
    if !out.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("lsblk failed: {}", String::from_utf8_lossy(&out.stderr)),
        ));
    }
    let parsed: Lsblk = serde_json::from_slice(&out.stdout)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

    Ok(parsed
        .blockdevices
        .into_iter()
        .filter(|dev| dev.tran.as_deref() == Some("usb"))
        .map(|dev| {
            let partitions: Vec<Partition> = dev
                .children
                .unwrap_or_default()
                .into_iter()
                .filter_map(|child| {
                    let mountpoint = first_mountpoint(&child);
                    Some(Partition {
                        path: child.path?,
                        label: child.label,
                        fstype: child.fstype,
                        mountpoint,
                    })
                })
                .collect();
            let mounted = partitions.iter().any(|part| part.mountpoint.is_some());
            UsbDrive {
                device: dev.path.unwrap_or_default(),
                model: dev.model.unwrap_or_else(|| "USB drive".into()).trim().to_string(),
                size_bytes: dev.size.unwrap_or(0),
                partitions,
                mounted,
            }
        })
        .filter(|drive| !drive.device.is_empty())
        .collect())
}

fn first_mountpoint(dev: &Device) -> Option<String> {
    dev.mountpoints
        .as_ref()
        .and_then(|points| points.iter().flatten().next().cloned())
}

/// Locate the sync stick by its filesystem label (default KUBUNTU-USB).
/// Returns the drive, the labelled partition, and where it is mounted
/// (None when not currently mounted).
pub fn find_stick(
    label: &str,
) -> io::Result<Option<(UsbDrive, Partition, Option<String>)>> {
    for drive in list_usb_drives()? {
        let found = drive.find_label(label).cloned();
        if let Some(part) = found {
            let mount = part.mountpoint.clone();
            return Ok(Some((drive, part, mount)));
        }
    }
    Ok(None)
}
