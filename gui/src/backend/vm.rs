//! Boot the stick in QEMU. Mirrors vm/boot-stick.sh: UEFI via OVMF, KVM
//! when available, the whole-disk USB device attached as a raw drive, and
//! a hard refusal for non-USB targets.

use crate::backend::drives::UsbDrive;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

const OVMF_CODE: &str = "/usr/share/OVMF/OVMF_CODE.fd";
const OVMF_VARS: &str = "/usr/share/OVMF/OVMF_VARS.fd";

/// Why a VM cannot be started right now.
#[derive(Debug)]
pub enum BootBlocker {
    NotUsbDrive,
    DriveMounted,
    NoOvmf,
    Io(io::Error),
}

impl std::fmt::Display for BootBlocker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BootBlocker::NotUsbDrive => write!(f, "refusing: target is not a USB device"),
            BootBlocker::DriveMounted => write!(
                f,
                "the stick is mounted - unmount it before booting it in a VM"
            ),
            BootBlocker::NoOvmf => write!(f, "missing OVMF firmware (install the ovmf package)"),
            BootBlocker::Io(err) => write!(f, "{err}"),
        }
    }
}

pub struct VmHandle {
    child: Child,
}

impl VmHandle {
    pub fn is_running(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    pub fn stop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for VmHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

fn kvm_args() -> Vec<String> {
    if Path::new("/dev/kvm").exists() {
        vec!["-enable-kvm".into(), "-cpu".into(), "host".into()]
    } else {
        // Software emulation: slow but works everywhere.
        Vec::new()
    }
}

fn ovmf_vars_copy() -> Result<PathBuf, BootBlocker> {
    let vars = crate::config::cache_dir().join("ovmf-vars.fd");
    if !vars.exists() {
        std::fs::create_dir_all(vars.parent().unwrap()).map_err(BootBlocker::Io)?;
        std::fs::copy(OVMF_VARS, &vars).map_err(BootBlocker::Io)?;
    }
    Ok(vars)
}

fn base_command() -> Result<Command, BootBlocker> {
    if !Path::new(OVMF_CODE).exists() {
        return Err(BootBlocker::NoOvmf);
    }
    let vars = ovmf_vars_copy()?;
    let mut cmd = Command::new("qemu-system-x86_64");
    cmd.args(kvm_args())
        .args(["-m", "4096", "-smp", "2"])
        .args(["-drive", &format!("if=pflash,format=raw,readonly=on,file={OVMF_CODE}")])
        .args(["-drive", &format!("if=pflash,format=raw,file={}", vars.display())])
        .args(["-usb", "-device", "usb-tablet"]);
    Ok(cmd)
}

/// Boot an installed stick (its own bootloader).
pub fn boot_stick(drive: &UsbDrive) -> Result<VmHandle, BootBlocker> {
    if drive.mounted {
        return Err(BootBlocker::DriveMounted);
    }
    let mut cmd = base_command()?;
    cmd.args([
        "-drive",
        &format!("file={},format=raw,if=ide,index=0,media=disk", drive.device),
    ]);
    let child = cmd.spawn().map_err(BootBlocker::Io)?;
    Ok(VmHandle { child })
}

/// Boot the Kubuntu installer ISO with the stick attached as the install
/// disk - this is step 2 of the flash wizard: the real Kubuntu installer
/// installs straight onto the stick.
pub fn boot_installer(drive: &UsbDrive, iso: &Path) -> Result<VmHandle, BootBlocker> {
    if drive.mounted {
        return Err(BootBlocker::DriveMounted);
    }
    let mut cmd = base_command()?;
    cmd.args([
        "-drive",
        &format!("file={},format=raw,if=ide,index=0,media=disk", drive.device),
    ])
    .args(["-cdrom", &iso.display().to_string()])
    .args(["-boot", "once=d"]);
    let child = cmd.spawn().map_err(BootBlocker::Io)?;
    Ok(VmHandle { child })
}
