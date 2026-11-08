//! kubuntu-usb-sync - flash a bootable Kubuntu USB stick and keep it in
//! sync with this PC. egui UI over a small CLI-tool backend (lsblk, unison,
//! qemu, curl).
//!
//! Modes:
//!   (default)    the GUI
//!   --watch      headless one-shot sync for the systemd user unit (udev
//!                starts it when the KUBUNTU-USB stick is plugged in)
//!
//! Debug flags (used to capture screenshots of every state):
//!   --screen folders|apps|flash|vm|sync|settings   start on a screen
//!   --add-dialog                                   open the Add-folder dialog
//!   --erase-dialog                                 open the erase confirmation dialog
//!   --vm-running                                   start the VM screen in "running" state
//!   --demo-sync                                    demo progress + conflict on the Sync screen
//!   --demo-drives                                  pretend a SanDisk Ultra stick is plugged in

use eframe::egui::{
    self, Align, Color32, ComboBox, CornerRadius, Frame, Layout, Margin, ProgressBar, RichText,
    ScrollArea, Stroke, TextEdit, Vec2,
};

mod backend;
mod config;

use backend::{apps, drives, flash, unison, vm};
use config::{FolderConfig as Folder, Settings, SyncMode as Mode};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// KDE Breeze dark palette
// ---------------------------------------------------------------------------
const BG: Color32 = Color32::from_rgb(0x1b, 0x1e, 0x20); // window background
const PANEL: Color32 = Color32::from_rgb(0x23, 0x26, 0x29); // panels / cards
const PANEL_HI: Color32 = Color32::from_rgb(0x2b, 0x2f, 0x33); // hover-ish panel
const BORDER: Color32 = Color32::from_rgb(0x4d, 0x4d, 0x4d); // subtle Breeze border
const ACCENT: Color32 = Color32::from_rgb(0x3d, 0xae, 0xe9); // Breeze blue
const DANGER: Color32 = Color32::from_rgb(0xda, 0x44, 0x53); // Breeze red
const GREEN: Color32 = Color32::from_rgb(0x2e, 0xb8, 0x6e);
const TEXT: Color32 = Color32::from_rgb(0xef, 0xf0, 0xf1);
const TEXT_DIM: Color32 = Color32::from_rgb(0x9d, 0xa0, 0xa3);

// ---------------------------------------------------------------------------
// Screens
// ---------------------------------------------------------------------------
#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Folders,
    Apps,
    FlashStick,
    VirtualMachine,
    Sync,
    Settings,
}

impl Screen {
    const ALL: [Screen; 6] = [
        Screen::Folders,
        Screen::Apps,
        Screen::FlashStick,
        Screen::VirtualMachine,
        Screen::Sync,
        Screen::Settings,
    ];

    fn label(self) -> &'static str {
        match self {
            Screen::Folders => "Folders",
            Screen::Apps => "Apps",
            Screen::FlashStick => "Flash Stick",
            Screen::VirtualMachine => "Virtual Machine",
            Screen::Sync => "Sync",
            Screen::Settings => "Settings",
        }
    }
}

// ---------------------------------------------------------------------------
// Dummy data
// ---------------------------------------------------------------------------
struct AppEntry {
    name: String,
    flatpak: bool,
    selected: bool,
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------
struct KubuntuUsbSyncApp {
    screen: Screen,
    folders: Vec<Folder>,
    apps: Vec<AppEntry>,
    add_dialog_open: bool,
    add_dialog_path: String,
    erase_confirm_open: bool,
    vm_running: bool,
    copy_user: String,
    auto_detect: bool,
    auto_sync: bool,
    last_synced: Option<String>,
    stick_label: String,
    demo_drives: bool,
    // ---- backend state ----
    drives: Vec<drives::UsbDrive>,
    drives_rx: Option<Receiver<Vec<drives::UsbDrive>>>,
    drives_refreshed: Instant,
    selected_drive: Option<String>,
    drive_chooser_open: bool,
    inventory_rx: Option<Receiver<Vec<apps::InventoryEntry>>>,
    inventory_loaded: bool,
    vm: Option<vm::VmHandle>,
    vm_status: Option<String>,
    sync_rx: Option<Receiver<unison::SyncEvent>>,
    resolve_rx: Option<Receiver<ResolveOutcome>>,
    sync_profiles: Vec<PathBuf>,
    sync_roots: Option<(String, String)>,
    sync_running: bool,
    sync_lines: Vec<String>,
    /// (profile index, path) pairs reported by unison.
    sync_conflicts: Vec<(usize, String)>,
    sync_done: usize,
    sync_total: usize,
    iso_rx: Option<Receiver<flash::DownloadEvent>>,
    iso_bytes: u64,
    iso_size: Option<u64>,
    iso_downloading: bool,
    iso_verifying: bool,
    flash_status: Option<String>,
    preseed_status: Option<String>,
}

/// Result of one Keep PC / Keep stick conflict resolution.
struct ResolveOutcome {
    conflict: (usize, String),
    prefer_pc: bool,
    result: Result<(), String>,
}

impl KubuntuUsbSyncApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        apply_breeze_style(&cc.egui_ctx);
        let flags = Flags::parse();
        let settings = Settings::load();
        // Demo list until the real apt/flatpak inventory arrives from the
        // startup worker (kept as the fallback when there is none).
        let apps = build_apps(demo_inventory(), &settings.selected_apps);
        let (inventory_tx, inventory_rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = inventory_tx.send(apps::inventory());
        });
        let iso_size = std::fs::metadata(flash::iso_path()).ok().map(|m| m.len());
        Self {
            screen: flags.screen,
            folders: settings.folders.clone(),
            apps,
            add_dialog_open: flags.add_dialog,
            add_dialog_path: String::new(),
            erase_confirm_open: flags.erase_dialog,
            vm_running: flags.vm_running,
            copy_user: settings.copy_user.clone(),
            auto_detect: settings.auto_detect,
            auto_sync: settings.auto_sync,
            last_synced: settings.last_synced.clone(),
            stick_label: settings.stick_label.clone(),
            demo_drives: flags.demo_drives,
            drives: Vec::new(),
            drives_rx: None,
            drives_refreshed: Instant::now() - Duration::from_secs(10),
            selected_drive: None,
            drive_chooser_open: false,
            inventory_rx: Some(inventory_rx),
            inventory_loaded: false,
            vm: None,
            vm_status: None,
            sync_rx: None,
            resolve_rx: None,
            sync_profiles: Vec::new(),
            sync_roots: None,
            sync_running: false,
            sync_lines: Vec::new(),
            sync_conflicts: Vec::new(),
            sync_done: 0,
            sync_total: 0,
            iso_rx: None,
            iso_bytes: 0,
            iso_size,
            iso_downloading: false,
            iso_verifying: false,
            flash_status: None,
            preseed_status: None,
        }
        .with_demo_sync(flags.demo_sync)
    }

    /// Demo Sync screen content for screenshots (--demo-sync).
    fn with_demo_sync(mut self, demo: bool) -> Self {
        if demo {
            self.sync_lines = vec!["Syncing ~/School … 120 of 184 files".into()];
            self.sync_conflicts = vec![(0, "School/notes.txt".into())];
            self.sync_done = 65;
            self.sync_total = 100;
        }
        self
    }

    /// Write the current UI state back to the TOML config.
    fn persist(&self) {
        let settings = Settings {
            folders: self.folders.clone(),
            auto_detect: self.auto_detect,
            auto_sync: self.auto_sync,
            copy_user: self.copy_user.clone(),
            selected_apps: self
                .apps
                .iter()
                .filter(|app| app.selected)
                .map(|app| app.name.to_string())
                .collect(),
            stick_label: self.stick_label.clone(),
            last_synced: self.last_synced.clone(),
        };
        if let Err(err) = settings.save() {
            eprintln!("settings: could not save: {err}");
        }
    }

    /// The drive the flash/VM screens act on: the picked one, else the
    /// first detected, else the demo drive when --demo-drives is set.
    fn current_drive(&self) -> Option<drives::UsbDrive> {
        self.selected_drive
            .as_ref()
            .and_then(|dev| self.drives.iter().find(|d| &d.device == dev))
            .or_else(|| self.drives.first())
            .cloned()
            .or_else(|| self.demo_drives.then(demo_drive))
    }

    /// Refresh the drive list on a worker thread (lsblk can take a moment;
    /// never block the UI thread on it).
    fn refresh_drives(&mut self) {
        if self.drives_rx.is_some() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(drives::list_usb_drives().unwrap_or_default());
        });
        self.drives_rx = Some(rx);
    }

    /// Kick off a real sync run in a worker thread.
    fn start_sync(&mut self) {
        if self.sync_running {
            return;
        }
        match prepare_sync(&self.folders, &self.copy_user, &self.stick_label) {
            Ok(prep) => {
                self.sync_total = prep.profiles.len();
                self.sync_done = 0;
                self.sync_lines.clear();
                self.sync_conflicts.clear();
                self.sync_roots = Some(prep.roots);
                self.sync_profiles = prep.profiles.clone();
                let (tx, rx) = mpsc::channel();
                std::thread::spawn(move || {
                    unison::run_sync(prep.profiles, &prep.backup_dir, tx);
                });
                self.sync_rx = Some(rx);
                self.sync_running = true;
            }
            Err(err) => {
                self.sync_lines = vec![err.to_string()];
            }
        }
    }

    /// Resolve one conflict by re-running unison on that path with -prefer.
    /// The conflict is removed from the list only when unison confirms.
    fn resolve_conflict(&mut self, index: usize, prefer_pc: bool) {
        let Some(conflict) = self.sync_conflicts.get(index).cloned() else {
            return;
        };
        let Some((pc_root, stick_root)) = self.sync_roots.clone() else {
            return;
        };
        let Some(profile) = self.sync_profiles.get(conflict.0).cloned() else {
            self.sync_lines
                .push("internal error: no profile for that conflict".into());
            return;
        };
        let root = if prefer_pc { pc_root } else { stick_root };
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = unison::resolve_conflict(&profile, &root, &conflict.1)
                .map_err(|err| err.to_string());
            let _ = tx.send(ResolveOutcome {
                conflict,
                prefer_pc,
                result,
            });
        });
        self.resolve_rx = Some(rx);
    }

    /// Drain backend channels and refresh drive info; called every frame.
    fn poll_backend(&mut self, ctx: &egui::Context) {
        // Startup inventory (apt-mark/flatpak) arrives from its worker.
        if let Some(rx) = &self.inventory_rx {
            match rx.try_recv() {
                Ok(entries) => {
                    self.inventory_loaded = true;
                    if !entries.is_empty() {
                        let selected = Settings::load().selected_apps;
                        self.apps = build_apps(entries, &selected);
                    }
                    self.inventory_rx = None;
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => self.inventory_rx = None,
            }
        }

        // Drive list refreshes on a worker; poll the result channel.
        if matches!(self.screen, Screen::FlashStick | Screen::VirtualMachine)
            && self.drives_refreshed.elapsed() > Duration::from_secs(3)
        {
            self.refresh_drives();
            self.drives_refreshed = Instant::now();
            if !self.iso_downloading {
                self.iso_size = std::fs::metadata(flash::iso_path()).ok().map(|m| m.len());
            }
        }
        if let Some(rx) = &self.drives_rx {
            match rx.try_recv() {
                Ok(drives) => {
                    self.drives = drives;
                    self.drives_rx = None;
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => self.drives_rx = None,
            }
        }

        if let Some(rx) = &self.sync_rx {
            let mut finished = None;
            let mut disconnected = false;
            loop {
                match rx.try_recv() {
                    Ok(event) => match event {
                        unison::SyncEvent::Line(line) => {
                            self.sync_lines.push(line);
                            if self.sync_lines.len() > 200 {
                                self.sync_lines.drain(..100);
                            }
                        }
                        unison::SyncEvent::Conflict { profile, path } => {
                            self.sync_conflicts.push((profile, path));
                        }
                        unison::SyncEvent::FolderDone(_) => self.sync_done += 1,
                        unison::SyncEvent::Finished(clean) => finished = Some(clean),
                    },
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                }
            }
            // The worker always sends Finished; if the channel closed
            // without one, the worker died - stop waiting forever.
            if finished.is_none() && disconnected && self.sync_running {
                self.sync_lines
                    .push("sync worker stopped unexpectedly".into());
                finished = Some(false);
            }
            if let Some(clean) = finished {
                self.sync_running = false;
                self.sync_rx = None;
                if clean {
                    self.last_synced = Some(config::timestamp());
                    unison::prune_backups(&backup_root(), unison::BACKUP_RUNS_KEPT);
                    self.persist();
                }
            }
        }

        if let Some(rx) = &self.resolve_rx {
            match rx.try_recv() {
                Ok(outcome) => {
                    let (profile, path) = &outcome.conflict;
                    match outcome.result {
                        Ok(()) => {
                            self.sync_conflicts
                                .retain(|c| c != &(*profile, path.clone()));
                            let side = if outcome.prefer_pc { "PC" } else { "stick" };
                            self.sync_lines
                                .push(format!("Resolved {path} (kept the {side} copy)"));
                        }
                        Err(err) => {
                            self.sync_lines
                                .push(format!("Could not resolve {path}: {err}"));
                        }
                    }
                    self.resolve_rx = None;
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => self.resolve_rx = None,
            }
        }

        if let Some(rx) = &self.iso_rx {
            let mut close = false;
            while let Ok(event) = rx.try_recv() {
                match event {
                    flash::DownloadEvent::Progress(bytes) => self.iso_bytes = bytes,
                    flash::DownloadEvent::Verifying => self.iso_verifying = true,
                    flash::DownloadEvent::Done(path) => {
                        self.iso_downloading = false;
                        self.iso_verifying = false;
                        self.iso_size = std::fs::metadata(&path).ok().map(|m| m.len());
                        close = true;
                    }
                    flash::DownloadEvent::Failed(msg) => {
                        self.iso_downloading = false;
                        self.iso_verifying = false;
                        self.flash_status = Some(format!("ISO download failed: {msg}"));
                        close = true;
                    }
                }
            }
            if close {
                self.iso_rx = None;
            }
        }

        // Reflect the real VM process state.
        if let Some(handle) = self.vm.as_mut() {
            if !handle.is_running() {
                self.vm = None;
                self.vm_running = false;
            }
        }

        if self.sync_running || self.iso_downloading || self.vm.is_some() {
            ctx.request_repaint_after(Duration::from_millis(300));
        }
    }
}

/// Backup runs live under ~/.kubuntu-usb/backups/<timestamp>.
fn backup_root() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".kubuntu-usb/backups")
}

/// Everything a sync run needs: written profiles, the PC/stick roots
/// (for conflict resolution), and this run's backup directory.
struct PreparedSync {
    profiles: Vec<PathBuf>,
    roots: (String, String),
    backup_dir: PathBuf,
}

/// Why a sync cannot start.
#[derive(Debug)]
enum PrepError {
    /// No stick with this label plugged in (not an error for the
    /// --watch unit: plugging the stick in is what starts it).
    NotFound(String),
    NotMounted,
    Other(String),
}

impl std::fmt::Display for PrepError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrepError::NotFound(label) => {
                write!(f, "No stick with the label {label} found - plug it in.")
            }
            PrepError::NotMounted => {
                write!(f, "Stick found but not mounted - mount it in Dolphin first.")
            }
            PrepError::Other(msg) => write!(f, "{msg}"),
        }
    }
}

/// Shared sync setup for the GUI "Sync now" button and the --watch unit:
/// locate the stick, write the unison profiles, name the backup dir.
fn prepare_sync(
    folders: &[Folder],
    copy_user: &str,
    stick_label: &str,
) -> Result<PreparedSync, PrepError> {
    let home = std::env::var("HOME").unwrap_or_default();
    let (_drive, _part, mount) = drives::find_stick(stick_label)
        .map_err(|err| PrepError::Other(format!("could not list drives: {err}")))?
        .ok_or_else(|| PrepError::NotFound(stick_label.to_string()))?;
    let mount = mount.ok_or(PrepError::NotMounted)?;
    let stick_home = format!("{mount}/home/{copy_user}");
    let dir = config::cache_dir().join("profiles");
    let profiles = unison::write_profiles(folders, &home, &stick_home, &dir)
        .map_err(|err| PrepError::Other(format!("could not write profiles: {err}")))?;
    let backup_dir = backup_root().join(config::timestamp().replace(' ', "_").replace(':', "-"));
    Ok(PreparedSync {
        profiles,
        roots: (home, stick_home),
        backup_dir,
    })
}

/// Placeholder stick for screenshots and demos (--demo-drives only).
fn demo_drive() -> drives::UsbDrive {
    drives::UsbDrive {
        device: "/dev/sdb".into(),
        model: "SanDisk Ultra".into(),
        size_bytes: 34_359_738_368, // 32 GiB, matching the approved mock
        partitions: Vec::new(),
        mounted: false,
    }
}

/// Demo inventory used until the real apt/flatpak list arrives, and as
/// the fallback on systems without either.
fn demo_inventory() -> Vec<apps::InventoryEntry> {
    [
        ("Firefox", false), ("Thunderbird", false), ("Kate", false),
        ("htop", false), ("git", false), ("VLC", true), ("GIMP", true),
        ("Discord", true), ("OBS Studio", true),
    ]
    .into_iter()
    .map(|(name, flatpak)| apps::InventoryEntry {
        id: name.to_string(),
        source: if flatpak { apps::AppSource::Flatpak } else { apps::AppSource::Apt },
    })
    .collect()
}

/// Merge an inventory with the saved selection. Ticked apps that are
/// not installed on this PC stay in the list (and in selected_apps) -
/// they are meant for the stick, not for here.
fn build_apps(entries: Vec<apps::InventoryEntry>, selected: &[String]) -> Vec<AppEntry> {
    let pick = |name: &str| selected.iter().any(|s| s.eq_ignore_ascii_case(name));
    let mut list: Vec<AppEntry> = entries
        .into_iter()
        .map(|entry| AppEntry {
            selected: pick(&entry.id),
            flatpak: entry.source == apps::AppSource::Flatpak,
            name: entry.id,
        })
        .collect();
    for name in selected {
        if !list.iter().any(|app| app.name.eq_ignore_ascii_case(name)) {
            // Flatpak IDs are dotted (org.videolan.VLC); apt names are not.
            let flatpak = name.contains('.');
            list.push(AppEntry {
                name: name.clone(),
                flatpak,
                selected: true,
            });
        }
    }
    list
}

// ---------------------------------------------------------------------------
// Debug flags for screenshots
// ---------------------------------------------------------------------------
struct Flags {
    screen: Screen,
    add_dialog: bool,
    erase_dialog: bool,
    vm_running: bool,
    demo_sync: bool,
    demo_drives: bool,
}

impl Flags {
    fn parse() -> Self {
        let mut flags = Flags {
            screen: Screen::Folders,
            add_dialog: false,
            erase_dialog: false,
            vm_running: false,
            demo_sync: false,
            demo_drives: false,
        };
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--screen" => {
                    if let Some(name) = args.next() {
                        flags.screen = match name.as_str() {
                            "folders" => Screen::Folders,
                            "apps" => Screen::Apps,
                            "flash" => Screen::FlashStick,
                            "vm" => Screen::VirtualMachine,
                            "sync" => Screen::Sync,
                            "settings" => Screen::Settings,
                            _ => Screen::Folders,
                        };
                    }
                }
                "--add-dialog" => flags.add_dialog = true,
                "--erase-dialog" => flags.erase_dialog = true,
                "--vm-running" => flags.vm_running = true,
                "--demo-sync" => flags.demo_sync = true,
                "--demo-drives" => flags.demo_drives = true,
                _ => {}
            }
        }
        flags
    }
}

// ---------------------------------------------------------------------------
// Styling helpers
// ---------------------------------------------------------------------------
fn apply_breeze_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    let mut v = egui::Visuals::dark();
    v.window_fill = PANEL;
    v.panel_fill = BG;
    v.extreme_bg_color = Color32::from_rgb(0x14, 0x16, 0x18);
    v.faint_bg_color = PANEL_HI;
    v.selection.bg_fill = ACCENT;
    v.hyperlink_color = ACCENT;
    v.widgets.inactive.bg_fill = PANEL_HI;
    v.widgets.inactive.weak_bg_fill = PANEL_HI;
    v.widgets.inactive.corner_radius = CornerRadius::same(4);
    v.widgets.hovered.bg_fill = Color32::from_rgb(0x31, 0x36, 0x3b);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0x31, 0x36, 0x3b);
    v.widgets.active.bg_fill = ACCENT;
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT);
    v.window_stroke = Stroke::new(1.0_f32, BORDER);
    style.visuals = v;
    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    ctx.set_style(style);
}

fn page_heading(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.label(RichText::new(title).size(24.0).strong().color(TEXT));
    if !subtitle.is_empty() {
        ui.label(RichText::new(subtitle).size(13.0).color(TEXT_DIM));
    }
    ui.add_space(14.0);
}

fn card(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    Frame::new()
        .fill(PANEL)
        .corner_radius(CornerRadius::same(8))
        .stroke(Stroke::new(1.0_f32, BORDER))
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            let w = ui.available_width();
            ui.set_min_width(w);
            add_contents(ui);
        });
    ui.add_space(10.0);
}

fn mode_button(ui: &mut egui::Ui, label: &str, active: bool) -> bool {
    ui.add(
        egui::Button::new(
            RichText::new(label)
                .size(12.5)
                .color(if active { Color32::WHITE } else { TEXT_DIM }),
        )
        .fill(if active { ACCENT } else { PANEL_HI })
        .stroke(Stroke::new(
            1.0_f32,
            if active { ACCENT } else { BORDER },
        ))
        .corner_radius(CornerRadius::same(4))
        .min_size(Vec2::new(96.0, 28.0)),
    )
    .clicked()
}

fn ghost_button(label: &str) -> egui::Button<'static> {
    egui::Button::new(RichText::new(label).color(TEXT_DIM))
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::new(1.0_f32, Color32::from_gray(90)))
        .corner_radius(CornerRadius::same(5))
        .min_size(Vec2::new(0.0, 34.0))
}

fn danger_button(label: &str) -> egui::Button<'static> {
    egui::Button::new(RichText::new(label).color(Color32::WHITE).strong())
        .fill(DANGER)
        .corner_radius(CornerRadius::same(5))
        .min_size(Vec2::new(0.0, 40.0))
}

fn accent_button(label: &str, height: f32) -> egui::Button<'static> {
    egui::Button::new(RichText::new(label).size(15.0).color(Color32::WHITE).strong())
        .fill(ACCENT)
        .corner_radius(CornerRadius::same(5))
        .min_size(Vec2::new(0.0, height))
}

fn setting_toggle(ui: &mut egui::Ui, label: &str, description: &str, on: &mut bool) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new(label).size(14.5).color(TEXT));
            ui.label(RichText::new(description).small().color(TEXT_DIM));
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let text = if *on { "On" } else { "Off" };
            let btn = egui::Button::new(
                RichText::new(text).color(if *on { Color32::WHITE } else { TEXT_DIM }),
            )
            .fill(if *on { ACCENT } else { PANEL_HI })
            .corner_radius(CornerRadius::same(10))
            .min_size(Vec2::new(56.0, 24.0));
            if ui.add(btn).clicked() {
                *on = !*on;
            }
        });
    });
}

// ---------------------------------------------------------------------------
// Screens
// ---------------------------------------------------------------------------
impl KubuntuUsbSyncApp {
    fn sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("nav")
            .exact_width(200.0)
            .resizable(false)
            .frame(
                Frame::new()
                    .fill(PANEL)
                    .inner_margin(Margin {
                        left: 8,
                        right: 8,
                        top: 14,
                        bottom: 10,
                    }),
            )
            .show(ctx, |ui| {
                ui.label(
                    RichText::new("Kubuntu\nUSB Sync")
                        .size(17.0)
                        .strong()
                        .color(ACCENT),
                );
                ui.label(RichText::new("Sync & flash tool").small().color(TEXT_DIM));
                ui.add_space(18.0);

                for screen in Screen::ALL {
                    let selected = self.screen == screen;
                    let text = RichText::new(format!("  {}", screen.label()))
                        .size(14.5)
                        .color(if selected {
                            Color32::WHITE
                        } else {
                            Color32::from_rgb(0xbd, 0xc0, 0xc3)
                        });
                    let btn = egui::Button::new(text)
                        .fill(if selected { ACCENT } else { Color32::TRANSPARENT })
                        .corner_radius(CornerRadius::same(6))
                        .min_size(Vec2::new(ui.available_width(), 38.0));
                    if ui.add(btn).clicked() {
                        self.screen = screen;
                    }
                    ui.add_space(2.0);
                }

                ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                    ui.label(RichText::new(format!("v{}", env!("CARGO_PKG_VERSION"))).small().color(TEXT_DIM));
                });
            });
    }

    fn show_folders(&mut self, ui: &mut egui::Ui) {
        page_heading(
            ui,
            "Folders",
            "These folders stay in sync between this PC and the stick.",
        );

        let mut changed = false;
        for i in 0..self.folders.len() {
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(&self.folders[i].path)
                                .size(15.0)
                                .strong()
                                .color(TEXT),
                        );
                        ui.label(
                            RichText::new(&self.folders[i].description)
                                .small()
                                .color(TEXT_DIM),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if mode_button(ui, "Stick wins", self.folders[i].mode == Mode::StickWins) {
                            self.folders[i].mode = Mode::StickWins;
                            changed = true;
                        }
                        if mode_button(ui, "PC wins", self.folders[i].mode == Mode::PcWins) {
                            self.folders[i].mode = Mode::PcWins;
                            changed = true;
                        }
                        if mode_button(
                            ui,
                            "↔ Both ways",
                            self.folders[i].mode == Mode::BothWays,
                        ) {
                            self.folders[i].mode = Mode::BothWays;
                            changed = true;
                        }
                    });
                });
            });
        }
        if changed {
            self.persist();
        }

        let add = egui::Button::new(RichText::new("+ Add folder…").size(14.0).color(TEXT_DIM))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::new(1.0_f32, Color32::from_gray(90)))
            .corner_radius(CornerRadius::same(8))
            .min_size(Vec2::new(ui.available_width(), 40.0));
        if ui.add(add).clicked() {
            self.add_dialog_open = true;
        }
    }

    fn show_apps(&mut self, ui: &mut egui::Ui) {
        page_heading(
            ui,
            "Apps",
            "Pick the apps that should be installed on the stick.",
        );
        let mut dirty = false;

        let count = self.apps.iter().filter(|a| a.selected).count();
        let summary = if count == 1 {
            "1 app will be installed on the stick".to_string()
        } else {
            format!("{count} apps will be installed on the stick")
        };
        card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(summary)
                        .size(14.0)
                        .strong()
                        .color(ACCENT),
                );
            });
        });

        card(ui, |ui| {
            ui.label(
                RichText::new("apt")
                    .size(15.0)
                    .strong()
                    .color(ACCENT),
            );
            ui.label(RichText::new("System packages").small().color(TEXT_DIM));
            ui.add_space(6.0);
            for app in self.apps.iter_mut().filter(|a| !a.flatpak) {
                if ui
                    .checkbox(
                        &mut app.selected,
                        RichText::new(app.name.as_str()).size(14.0).color(TEXT),
                    )
                    .changed()
                {
                    dirty = true;
                }
            }
        });

        card(ui, |ui| {
            ui.label(
                RichText::new("Flatpak")
                    .size(15.0)
                    .strong()
                    .color(ACCENT),
            );
            ui.label(RichText::new("From Flathub").small().color(TEXT_DIM));
            ui.add_space(6.0);
            for app in self.apps.iter_mut().filter(|a| a.flatpak) {
                if ui
                    .checkbox(
                        &mut app.selected,
                        RichText::new(app.name.as_str()).size(14.0).color(TEXT),
                    )
                    .changed()
                {
                    dirty = true;
                }
            }
        });
        if dirty {
            self.persist();
        }
    }

    fn show_flash_stick(&mut self, ui: &mut egui::Ui) {
        page_heading(
            ui,
            "Flash Stick",
            "Create a bootable Kubuntu stick with your apps and settings.",
        );

        // Drive card: the detected drive, or an honest empty state.
        let drive = self.current_drive();
        match &drive {
            Some(d) => {
                let (name, device, size) = (
                    d.display_name(),
                    d.device.clone(),
                    format!("{:.0} GB", d.size_gib()),
                );
                card(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new(name).size(16.0).strong().color(TEXT));
                            ui.label(RichText::new(device).monospace().color(TEXT_DIM));
                            ui.label(
                                RichText::new("Everything on the stick will be erased.")
                                    .small()
                                    .color(DANGER),
                            );
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(RichText::new(size).size(14.0).color(TEXT_DIM));
                        });
                    });
                });
            }
            None => {
                card(ui, |ui| {
                    ui.label(
                        RichText::new("No USB drive detected.")
                            .size(15.0)
                            .strong()
                            .color(TEXT_DIM),
                    );
                    ui.label(
                        RichText::new("Plug the stick in - it shows up here automatically.")
                            .small()
                            .color(TEXT_DIM),
                    );
                });
            }
        }

        // Step 1 - ISO download (cached metadata + verified download)
        card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("1.").size(15.0).strong().color(TEXT_DIM));
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Download Kubuntu 24.04 ISO")
                            .size(14.5)
                            .strong()
                            .color(TEXT),
                    );
                    if let Some(bytes) = self.iso_size.filter(|_| !self.iso_downloading) {
                        ui.label(
                            RichText::new(format!(
                                "Done - the ISO is downloaded and verified ({:.1} GB)",
                                bytes as f64 / 1e9
                            ))
                            .small()
                            .strong()
                            .color(GREEN),
                        );
                    } else if self.iso_verifying {
                        ui.label(
                            RichText::new("Verifying checksum ...")
                                .small()
                                .color(ACCENT),
                        );
                    } else if self.iso_downloading {
                        ui.label(
                            RichText::new(format!(
                                "Downloading ... {:.0} MB",
                                self.iso_bytes as f64 / 1e6
                            ))
                            .small()
                            .color(ACCENT),
                        );
                    } else {
                        if ui
                            .add(accent_button("Download ISO", 30.0).min_size(Vec2::new(140.0, 30.0)))
                            .clicked()
                        {
                            let (tx, rx) = mpsc::channel();
                            std::thread::spawn(move || flash::download_iso(tx));
                            self.iso_rx = Some(rx);
                            self.iso_downloading = true;
                            self.iso_bytes = 0;
                        }
                    }
                });
            });
        });

        // Step 2
        card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("2.").size(15.0).strong().color(TEXT_DIM));
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Boot the ISO in a virtual machine")
                            .size(14.5)
                            .strong()
                            .color(TEXT),
                    );
                    ui.label(
                        RichText::new(
                            "The stick is attached to the VM as the install disk, so the real \
                             Kubuntu installer installs straight onto it. Pick \"Minimal \
                             installation\" in the installer.",
                        )
                        .small()
                        .color(TEXT_DIM),
                    );
                });
            });
        });

        // Step 3
        card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("3.").size(15.0).strong().color(TEXT_DIM));
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Copy settings to the stick")
                            .size(14.5)
                            .strong()
                            .color(TEXT),
                    );
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Copy settings from user:").small().color(TEXT_DIM));
                        let prev_user = self.copy_user.clone();
                        ComboBox::from_id_salt("copy_user")
                            .selected_text(self.copy_user.clone())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.copy_user, "gladi".to_string(), "gladi");
                                ui.selectable_value(&mut self.copy_user, "paul".to_string(), "paul");
                            });
                        if self.copy_user != prev_user {
                            self.persist();
                        }
                    });
                    if ui
                        .add(
                            accent_button("Copy settings now", 28.0)
                                .min_size(Vec2::new(170.0, 28.0)),
                        )
                        .clicked()
                    {
                        match drives::find_stick(&self.stick_label) {
                            Ok(Some((_d, _p, Some(mount)))) => {
                                let pc_home =
                                    PathBuf::from(std::env::var("HOME").unwrap_or_default());
                                let stick_home =
                                    PathBuf::from(mount).join("home").join(&self.copy_user);
                                self.preseed_status =
                                    Some(match flash::preseed_settings(&stick_home, &pc_home) {
                                        Ok(()) => "KDE settings copied to the stick.".into(),
                                        Err(err) => format!("Could not copy settings: {err}"),
                                    });
                            }
                            _ => {
                                self.preseed_status =
                                    Some("Stick not found or not mounted.".into())
                            }
                        }
                    }
                });
            });
        });

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.add(danger_button("⚠  Erase stick & start")).clicked() {
                self.erase_confirm_open = true;
            }
            if ui.add(ghost_button("Choose different drive…")).clicked() {
                self.refresh_drives();
                self.drive_chooser_open = true;
            }
        });
        if let Some(status) = &self.flash_status {
            ui.add_space(4.0);
            ui.label(RichText::new(status).color(DANGER));
        }
        if let Some(status) = &self.preseed_status {
            ui.add_space(4.0);
            ui.label(RichText::new(status).color(ACCENT));
        }
    }

    fn show_vm(&mut self, ui: &mut egui::Ui) {
        page_heading(
            ui,
            "Virtual Machine",
            "Test the stick in a VM before you boot from it for real.",
        );

        let drive = self.current_drive();
        card(ui, |ui| {
            ui.horizontal(|ui| match &drive {
                Some(d) => {
                    ui.label(RichText::new("■").small().color(GREEN));
                    ui.label(
                        RichText::new(format!("{} ({}) detected", d.display_name(), d.device))
                            .size(14.5)
                            .color(TEXT),
                    );
                }
                None => {
                    ui.label(
                        RichText::new("No USB drive detected - plug the stick in.")
                            .size(14.5)
                            .color(TEXT_DIM),
                    );
                }
            });
        });

        ui.add_space(4.0);
        let button_label = if self.vm_running {
            "■  Stop VM"
        } else {
            "▶  Boot stick in VM"
        };
        let button = if self.vm_running {
            danger_button(button_label).min_size(Vec2::new(240.0, 52.0))
        } else {
            accent_button(button_label, 52.0).min_size(Vec2::new(240.0, 52.0))
        };
        if ui.add(button).clicked() {
            if self.vm_running {
                if let Some(mut handle) = self.vm.take() {
                    handle.stop();
                }
                self.vm_running = false;
            } else if let Some(drive) = self.current_drive() {
                match vm::boot_stick(&drive) {
                    Ok(handle) => {
                        self.vm = Some(handle);
                        self.vm_running = true;
                        self.vm_status = None;
                    }
                    Err(blocker) => self.vm_status = Some(blocker.to_string()),
                }
            } else {
                self.vm_status = Some("No USB drive detected.".into());
            }
        }
        if let Some(status) = &self.vm_status {
            ui.label(RichText::new(status).color(DANGER));
        }

        ui.add_space(6.0);
        if self.vm_running {
            ui.label(
                RichText::new("VM running - the stick is attached as the install disk")
                    .color(ACCENT),
            );
        } else {
            ui.label(RichText::new("VM not running").color(TEXT_DIM));
        }

        ui.add_space(10.0);
        // Fake VM display
        Frame::new()
            .fill(Color32::from_rgb(0x10, 0x12, 0x13))
            .corner_radius(CornerRadius::same(8))
            .stroke(Stroke::new(1.0_f32, BORDER))
            .inner_margin(Margin::same(14))
            .show(ui, |ui| {
                ui.set_min_size(Vec2::new(ui.available_width(), 230.0));
                if self.vm_running {
                    ui.label(
                        RichText::new(
                            "GNU GRUB  version 2.12\n\n\
                             \u{20} Kubuntu 24.04 LTS\n\n\
                             Booting `Kubuntu 24.04 LTS` ...\n\n\
                             [  OK  ] Started QEMU Guest Agent.\n\
                             [  OK  ] Reached target Graphical Interface.\n\n\
                             Starting the Kubuntu installer (Minimal installation) ...",
                        )
                        .monospace()
                        .size(12.5)
                        .color(Color32::from_rgb(0x9a, 0xe6, 0x9a)),
                    );
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            RichText::new("VM display - not running")
                                .size(13.0)
                                .color(Color32::from_gray(80)),
                        );
                    });
                }
            });
    }

    fn show_sync(&mut self, ui: &mut egui::Ui) {
        page_heading(ui, "Sync", "Bring this PC and the stick to the same state.");

        let last = self
            .last_synced
            .clone()
            .unwrap_or_else(|| "never".to_string());
        ui.label(RichText::new(format!("Last synced: {last}")).color(TEXT_DIM));
        ui.add_space(6.0);
        let sync_label = if self.sync_running { "Syncing …" } else { "Sync now" };
        if ui
            .add_enabled(
                !self.sync_running,
                accent_button(sync_label, 48.0).min_size(Vec2::new(220.0, 48.0)),
            )
            .clicked()
        {
            self.start_sync();
        }

        ui.add_space(14.0);
        if self.sync_running || !self.sync_lines.is_empty() {
            card(ui, |ui| {
                ui.label(RichText::new("Progress").size(14.5).strong().color(TEXT));
                let fraction = if self.sync_total > 0 {
                    self.sync_done as f32 / self.sync_total as f32
                } else {
                    0.0
                };
                ui.add(
                    ProgressBar::new(fraction)
                        .fill(ACCENT)
                        .show_percentage()
                        .desired_height(18.0),
                );
                if let Some(line) = self.sync_lines.last() {
                    ui.label(RichText::new(line).small().color(TEXT_DIM));
                }
            });
        }

        if !self.sync_conflicts.is_empty() {
            let mut action: Option<(usize, bool)> = None;
            card(ui, |ui| {
                ui.label(
                    RichText::new(format!(
                        "{} conflict(s) need your decision",
                        self.sync_conflicts.len()
                    ))
                    .size(14.5)
                    .strong()
                    .color(DANGER),
                );
                ui.add_space(4.0);
                for (i, (_profile, path)) in self.sync_conflicts.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(format!("~/{path}"))
                                    .monospace()
                                    .size(13.5)
                                    .color(TEXT),
                            );
                            ui.label(
                                RichText::new("Changed on both sides")
                                    .small()
                                    .color(TEXT_DIM),
                            );
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.add(ghost_button("Keep stick")).clicked() {
                                action = Some((i, false));
                            }
                            if ui
                                .add(accent_button("Keep PC", 30.0).min_size(Vec2::new(90.0, 30.0)))
                                .clicked()
                            {
                                action = Some((i, true));
                            }
                        });
                    });
                }
            });
            if let Some((index, prefer_pc)) = action {
                self.resolve_conflict(index, prefer_pc);
            }
        }
    }

    fn show_settings(&mut self, ui: &mut egui::Ui) {
        page_heading(ui, "Settings", "How the app behaves.");
        let before = (self.auto_detect, self.auto_sync);

        card(ui, |ui| {
            setting_toggle(
                ui,
                "Detect stick automatically",
                "Notice the stick as soon as it is plugged in.",
                &mut self.auto_detect,
            );
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);
            setting_toggle(
                ui,
                "Auto-sync on plug-in",
                "Start syncing right away when the stick shows up.",
                &mut self.auto_sync,
            );
        });

        if before != (self.auto_detect, self.auto_sync) {
            self.persist();
        }

        card(ui, |ui| {
            ui.label(RichText::new("Configuration").size(14.5).strong().color(TEXT));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Config file:").color(TEXT_DIM));
                ui.label(
                    RichText::new(config::config_path().display().to_string())
                        .monospace()
                        .color(TEXT),
                );
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Stick label:").color(TEXT_DIM));
                ui.label(RichText::new(&self.stick_label).monospace().color(TEXT));
            });
            ui.label(
                RichText::new(
                    "The stick's filesystem label. If you change it in the config, update \
                     the udev rule (99-kubuntu-usb-sync.rules) too - auto-sync on plug-in \
                     matches this label.",
                )
                .small()
                .color(TEXT_DIM),
            );
            ui.add_space(6.0);
            if ui.add(ghost_button("Open config folder")).clicked() {
                let _ = std::process::Command::new("xdg-open")
                    .arg(config::config_dir())
                    .spawn();
            }
        });
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        if self.add_dialog_open {
            egui::Window::new("Add folder")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(RichText::new("Folder path:").color(TEXT_DIM));
                    ui.add(
                        TextEdit::singleline(&mut self.add_dialog_path)
                            .desired_width(380.0)
                            .hint_text("/home/gladi/Documents"),
                    );
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui
                                .add(accent_button("Add", 30.0).min_size(Vec2::new(80.0, 30.0)))
                                .clicked()
                            {
                                if !self.add_dialog_path.trim().is_empty() {
                                    self.folders.push(Folder {
                                        path: self.add_dialog_path.trim().to_string(),
                                        description: "Added manually".into(),
                                        mode: Mode::BothWays,
                                    });
                                    self.persist();
                                }
                                self.add_dialog_path.clear();
                                self.add_dialog_open = false;
                            }
                            if ui.add(ghost_button("Cancel")).clicked() {
                                self.add_dialog_open = false;
                            }
                        });
                    });
                });
        }

        if self.erase_confirm_open {
            let drive = self.current_drive();
            egui::Window::new("Erase stick?")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    match &drive {
                        Some(d) => ui.label(
                            RichText::new(format!(
                                "This will permanently erase {}. Continue?",
                                d.device
                            ))
                            .size(14.0)
                            .color(TEXT),
                        ),
                        None => ui.label(
                            RichText::new("No USB drive detected - plug the stick in first.")
                                .size(14.0)
                                .color(DANGER),
                        ),
                    };
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if drive.is_some()
                                && ui
                                    .add(danger_button("Erase").min_size(Vec2::new(80.0, 32.0)))
                                    .clicked()
                            {
                                self.erase_confirm_open = false;
                                let iso = flash::iso_path();
                                match drive {
                                    Some(drive) if self.iso_size.is_none() => {
                                        let _ = drive;
                                        self.flash_status =
                                            Some("Download the ISO first (step 1).".into());
                                    }
                                    Some(drive) => match vm::boot_installer(&drive, &iso) {
                                        Ok(handle) => {
                                            self.vm = Some(handle);
                                            self.vm_running = true;
                                            self.flash_status = Some(
                                                "Installer booted in the VM - install Kubuntu \
                                                 onto the stick (Minimal installation)."
                                                    .into(),
                                            );
                                        }
                                        Err(blocker) => {
                                            self.flash_status = Some(blocker.to_string())
                                        }
                                    },
                                    None => {
                                        self.flash_status = Some("No USB drive detected.".into())
                                    }
                                }
                            }
                            if ui.add(ghost_button("Cancel")).clicked() {
                                self.erase_confirm_open = false;
                            }
                        });
                    });
                });
        }

        if self.drive_chooser_open {
            egui::Window::new("Choose drive")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    if self.drives.is_empty() {
                        ui.label(
                            RichText::new("No USB drives detected. Plug the stick in.")
                                .color(TEXT_DIM),
                        );
                    }
                    let mut pick = None;
                    for drive in &self.drives {
                        let label = format!("{}  -  {}", drive.display_name(), drive.device);
                        if ui
                            .add(ghost_button(&label).min_size(Vec2::new(340.0, 32.0)))
                            .clicked()
                        {
                            pick = Some(drive.device.clone());
                        }
                        ui.add_space(2.0);
                    }
                    if let Some(device) = pick {
                        self.selected_drive = Some(device);
                        self.drive_chooser_open = false;
                    }
                    ui.add_space(6.0);
                    if ui.add(ghost_button("Cancel")).clicked() {
                        self.drive_chooser_open = false;
                    }
                });
        }
    }
}

impl eframe::App for KubuntuUsbSyncApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_backend(ctx);
        self.sidebar(ctx);
        egui::CentralPanel::default()
            .frame(Frame::new().fill(BG).inner_margin(Margin::same(22)))
            .show(ctx, |ui| {
                ScrollArea::vertical().show(ui, |ui| match self.screen {
                    Screen::Folders => self.show_folders(ui),
                    Screen::Apps => self.show_apps(ui),
                    Screen::FlashStick => self.show_flash_stick(ui),
                    Screen::VirtualMachine => self.show_vm(ui),
                    Screen::Sync => self.show_sync(ui),
                    Screen::Settings => self.show_settings(ui),
                });
            });
        self.dialogs(ctx);
    }
}

/// Headless one-shot sync, run by the systemd --user unit on plug-in.
/// Exit codes: 0 = synced or nothing to do, 1 = error, 2 = conflicts.
fn run_watch() -> i32 {
    let settings = Settings::load();
    if !settings.auto_sync {
        println!("auto-sync on plug-in is disabled in the settings");
        return 0;
    }
    match prepare_sync(&settings.folders, &settings.copy_user, &settings.stick_label) {
        Ok(prep) => {
            let (tx, rx) = mpsc::channel();
            let handle =
                std::thread::spawn(move || unison::run_sync(prep.profiles, &prep.backup_dir, tx));
            let mut code = 0;
            for event in rx {
                match event {
                    unison::SyncEvent::Line(line) => println!("{line}"),
                    unison::SyncEvent::Conflict { path, .. } => println!("CONFLICT: {path}"),
                    unison::SyncEvent::Finished(clean) => {
                        code = if clean { 0 } else { 2 };
                    }
                    unison::SyncEvent::FolderDone(_) => {}
                }
            }
            let _ = handle.join();
            if code == 0 {
                let mut settings = settings;
                settings.last_synced = Some(config::timestamp());
                let _ = settings.save();
                unison::prune_backups(&backup_root(), unison::BACKUP_RUNS_KEPT);
            }
            code
        }
        Err(PrepError::NotFound(label)) => {
            println!("no stick with the label {label} found - nothing to do");
            0
        }
        Err(err) => {
            eprintln!("{err}");
            1
        }
    }
}

fn main() -> eframe::Result<()> {
    if std::env::args().any(|arg| arg == "--watch") {
        std::process::exit(run_watch());
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Kubuntu USB Sync")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([1024.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Kubuntu USB Sync",
        options,
        Box::new(|cc| Ok(Box::new(KubuntuUsbSyncApp::new(cc)))),
    )
}
