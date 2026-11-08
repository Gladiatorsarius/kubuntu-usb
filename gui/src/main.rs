//! kubuntu-usb-sync - UI-only egui prototype.
//!
//! No backend and no working logic: buttons only navigate between screens,
//! open dialogs and toggle UI state. All data shown is dummy data.
//!
//! Debug flags (used to capture screenshots of every state):
//!   --screen folders|apps|flash|vm|sync|settings   start on a screen
//!   --add-dialog                                   open the Add-folder dialog
//!   --erase-dialog                                 open the erase confirmation dialog
//!   --vm-running                                   start the VM screen in "running" state

use eframe::egui::{
    self, Align, Color32, ComboBox, CornerRadius, Frame, Layout, Margin, ProgressBar, RichText,
    ScrollArea, Stroke, TextEdit, Vec2,
};

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
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    BothWays,
    PcWins,
    StickWins,
}

struct Folder {
    path: String,
    description: String,
    mode: Mode,
}

struct DummyApp {
    name: &'static str,
    flatpak: bool,
    selected: bool,
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------
struct KubuntuUsbSyncApp {
    screen: Screen,
    folders: Vec<Folder>,
    apps: Vec<DummyApp>,
    add_dialog_open: bool,
    add_dialog_path: String,
    erase_confirm_open: bool,
    vm_running: bool,
    copy_user: String,
    auto_detect: bool,
    auto_sync: bool,
}

impl KubuntuUsbSyncApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        apply_breeze_style(&cc.egui_ctx);
        let flags = Flags::parse();
        Self {
            screen: flags.screen,
            folders: vec![
                Folder {
                    path: "~/School".into(),
                    description: "Notes, homework and presentations".into(),
                    mode: Mode::BothWays,
                },
                Folder {
                    path: "~/.config".into(),
                    description: "Settings".into(),
                    mode: Mode::PcWins,
                },
                Folder {
                    path: "~/Projects".into(),
                    description: "Code and assets".into(),
                    mode: Mode::BothWays,
                },
            ],
            apps: vec![
                DummyApp { name: "Firefox", flatpak: false, selected: true },
                DummyApp { name: "Thunderbird", flatpak: false, selected: false },
                DummyApp { name: "Kate", flatpak: false, selected: true },
                DummyApp { name: "htop", flatpak: false, selected: true },
                DummyApp { name: "git", flatpak: false, selected: false },
                DummyApp { name: "VLC", flatpak: true, selected: true },
                DummyApp { name: "GIMP", flatpak: true, selected: false },
                DummyApp { name: "Discord", flatpak: true, selected: false },
                DummyApp { name: "OBS Studio", flatpak: true, selected: false },
            ],
            add_dialog_open: flags.add_dialog,
            add_dialog_path: String::new(),
            erase_confirm_open: flags.erase_dialog,
            vm_running: flags.vm_running,
            copy_user: "gladi".into(),
            auto_detect: true,
            auto_sync: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Debug flags for screenshots
// ---------------------------------------------------------------------------
struct Flags {
    screen: Screen,
    add_dialog: bool,
    erase_dialog: bool,
    vm_running: bool,
}

impl Flags {
    fn parse() -> Self {
        let mut flags = Flags {
            screen: Screen::Folders,
            add_dialog: false,
            erase_dialog: false,
            vm_running: false,
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
                ui.label(RichText::new("UI prototype").small().color(TEXT_DIM));
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
                    ui.label(RichText::new("v0.1.0 - UI mock").small().color(TEXT_DIM));
                });
            });
    }

    fn show_folders(&mut self, ui: &mut egui::Ui) {
        page_heading(
            ui,
            "Folders",
            "These folders stay in sync between this PC and the stick.",
        );

        let snapshot: Vec<(String, String, Mode)> = self
            .folders
            .iter()
            .map(|f| (f.path.clone(), f.description.clone(), f.mode))
            .collect();

        for (i, (path, description, mode)) in snapshot.iter().enumerate() {
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(path).size(15.0).strong().color(TEXT));
                        ui.label(RichText::new(description).small().color(TEXT_DIM));
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if mode_button(ui, "Stick wins", *mode == Mode::StickWins) {
                            self.folders[i].mode = Mode::StickWins;
                        }
                        if mode_button(ui, "PC wins", *mode == Mode::PcWins) {
                            self.folders[i].mode = Mode::PcWins;
                        }
                        if mode_button(ui, "↔ Both ways", *mode == Mode::BothWays) {
                            self.folders[i].mode = Mode::BothWays;
                        }
                    });
                });
            });
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
                ui.checkbox(&mut app.selected, RichText::new(app.name).size(14.0).color(TEXT));
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
                ui.checkbox(&mut app.selected, RichText::new(app.name).size(14.0).color(TEXT));
            }
        });

        let count = self.apps.iter().filter(|a| a.selected).count();
        card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("{count} apps will be installed on the stick"))
                        .size(14.0)
                        .strong()
                        .color(ACCENT),
                );
            });
        });
    }

    fn show_flash_stick(&mut self, ui: &mut egui::Ui) {
        page_heading(
            ui,
            "Flash Stick",
            "Create a bootable Kubuntu stick with your apps and settings.",
        );

        // Drive card
        card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("SanDisk Ultra 32 GB")
                            .size(16.0)
                            .strong()
                            .color(TEXT),
                    );
                    ui.label(RichText::new("/dev/sdb").monospace().color(TEXT_DIM));
                    ui.label(
                        RichText::new("The stick is empty - everything on it will be erased.")
                            .small()
                            .color(DANGER),
                    );
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new("32 GB").size(14.0).color(TEXT_DIM));
                });
            });
        });

        // Step 1 - done
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
                    ui.label(
                        RichText::new("Done - the ISO is already downloaded")
                            .small()
                            .strong()
                            .color(GREEN),
                    );
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
                        ComboBox::from_id_salt("copy_user")
                            .selected_text(self.copy_user.clone())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.copy_user, "gladi".to_string(), "gladi");
                                ui.selectable_value(&mut self.copy_user, "paul".to_string(), "paul");
                            });
                    });
                });
            });
        });

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.add(danger_button("⚠  Erase stick & start")).clicked() {
                self.erase_confirm_open = true;
            }
            ui.add(ghost_button("Choose different drive…"));
        });
    }

    fn show_vm(&mut self, ui: &mut egui::Ui) {
        page_heading(
            ui,
            "Virtual Machine",
            "Test the stick in a VM before you boot from it for real.",
        );

        card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("■").small().color(GREEN));
                ui.label(
                    RichText::new("SanDisk Ultra 32 GB (/dev/sdb) detected")
                        .size(14.5)
                        .color(TEXT),
                );
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
            self.vm_running = !self.vm_running;
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

        ui.label(RichText::new("Last synced: today 18:42").color(TEXT_DIM));
        ui.add_space(6.0);
        if ui
            .add(accent_button("Sync now", 48.0).min_size(Vec2::new(220.0, 48.0)))
            .clicked()
        {
            // UI mock: no real sync happens
        }

        ui.add_space(14.0);
        card(ui, |ui| {
            ui.label(RichText::new("Progress").size(14.5).strong().color(TEXT));
            ui.add(
                ProgressBar::new(0.65)
                    .fill(ACCENT)
                    .show_percentage()
                    .desired_height(18.0),
            );
            ui.label(
                RichText::new("Syncing ~/School … 120 of 184 files")
                    .small()
                    .color(TEXT_DIM),
            );
        });

        card(ui, |ui| {
            ui.label(
                RichText::new("1 conflict needs your decision")
                    .size(14.5)
                    .strong()
                    .color(DANGER),
            );
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("~/School/notes.txt")
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
                    ui.add(ghost_button("Keep stick"));
                    ui.add(accent_button("Keep PC", 30.0).min_size(Vec2::new(90.0, 30.0)));
                });
            });
        });
    }

    fn show_settings(&mut self, ui: &mut egui::Ui) {
        page_heading(ui, "Settings", "How the app behaves.");

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

        card(ui, |ui| {
            ui.label(RichText::new("Configuration").size(14.5).strong().color(TEXT));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Config file:").color(TEXT_DIM));
                ui.label(
                    RichText::new("~/.config/kubuntu-usb-sync/config.toml")
                        .monospace()
                        .color(TEXT),
                );
            });
            ui.add_space(6.0);
            ui.add(ghost_button("Open config folder"));
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
            egui::Window::new("Erase stick?")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(
                        RichText::new("This will permanently erase /dev/sdb. Continue?")
                            .size(14.0)
                            .color(TEXT),
                    );
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui
                                .add(danger_button("Erase").min_size(Vec2::new(80.0, 32.0)))
                                .clicked()
                            {
                                self.erase_confirm_open = false;
                            }
                            if ui.add(ghost_button("Cancel")).clicked() {
                                self.erase_confirm_open = false;
                            }
                        });
                    });
                });
        }
    }
}

impl eframe::App for KubuntuUsbSyncApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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

fn main() -> eframe::Result<()> {
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
