use crate::bridge::{AdbBridge, ScrcpyBridge};
use crate::config::AppConfig;
use crate::device::{get_devices, Device};
use crate::ui::{
    BottomPanel, DeviceList, SettingsWindow, SwipePanel, ToolkitPanel, WirelessAdbPanel,
};
use eframe::egui;
use egui::{Color32, RichText, Ui};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};

pub struct DroidViewApp {
    config: Arc<Mutex<AppConfig>>,
    devices: Vec<Device>,
    device_list: DeviceList,
    swipe_panel: SwipePanel,
    toolkit_panel: ToolkitPanel,
    bottom_panel: BottomPanel,
    wireless_adb_panel: WirelessAdbPanel,
    settings_window: SettingsWindow,
    adb_bridge: Option<AdbBridge>,
    scrcpy_bridge: Option<ScrcpyBridge>,
    status_message: String,
    scrcpy_running: bool,
    debug_disable_scrcpy: bool,
}

impl DroidViewApp {
    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        config: Arc<Mutex<AppConfig>>,
        debug_disable_scrcpy: bool,
    ) -> Self {
        let settings_window = SettingsWindow::new(config.clone());

        Self {
            config,
            devices: Vec::new(),
            device_list: DeviceList::new(),
            swipe_panel: SwipePanel::new(),
            toolkit_panel: ToolkitPanel::new(),
            bottom_panel: BottomPanel::new(),
            wireless_adb_panel: WirelessAdbPanel::new(),
            settings_window,
            adb_bridge: None,
            scrcpy_bridge: None,
            status_message: "Initializing...".to_string(),
            scrcpy_running: false,
            debug_disable_scrcpy,
        }
    }

    fn update_bridges(&mut self) {
        let mut config = self.config.try_lock().unwrap();

        // Auto-detect adb if not configured
        if config.adb_path.is_none() {
            if let Some(adb_path) = crate::utils::find_adb() {
                config.adb_path = Some(adb_path.to_string_lossy().to_string());
                info!(
                    "Auto-detected ADB at: {}",
                    config.adb_path.as_ref().unwrap()
                );
            }
        }

        // Auto-detect scrcpy if not configured
        if config.scrcpy_path.is_none() {
            if let Some(scrcpy_path) = crate::utils::find_scrcpy() {
                config.scrcpy_path = Some(scrcpy_path.to_string_lossy().to_string());
                info!(
                    "Auto-detected scrcpy at: {}",
                    config.scrcpy_path.as_ref().unwrap()
                );
            }
        }

        // Create ADB bridge
        if let Some(adb_path) = &config.adb_path {
            if self.adb_bridge.as_ref().map(|b| b.path()) != Some(adb_path.as_str()) {
                self.adb_bridge = Some(AdbBridge::new(adb_path.clone()));
            }
        }

        // Create scrcpy bridge
        if let Some(scrcpy_path) = &config.scrcpy_path {
            if self.scrcpy_bridge.as_ref().map(|b| b.path()) != Some(scrcpy_path.as_str()) {
                self.scrcpy_bridge = Some(ScrcpyBridge::new(scrcpy_path.clone()));
            }
        }
    }

    fn refresh_devices(&mut self) {
        if let Some(adb_bridge) = &self.adb_bridge {
            match get_devices(adb_bridge.path()) {
                Ok(devices) => {
                    self.devices = devices;
                    self.device_list.update_devices(self.devices.clone());
                    self.status_message = format!("Found {} device(s)", self.devices.len());
                }
                Err(e) => {
                    error!("Failed to get devices: {}", e);
                    self.status_message = format!("Error: {}", e);
                }
            }
        } else {
            self.status_message = "ADB not configured".to_string();
        }
    }

    fn update_scrcpy_status(&mut self) {
        use crate::utils::is_process_running;
        let was_running = self.scrcpy_running;
        self.scrcpy_running = is_process_running("scrcpy");

        // Log status changes for debugging
        if was_running != self.scrcpy_running {
            if self.scrcpy_running {
                info!("Scrcpy process detected as running");
            } else {
                info!("Scrcpy process no longer detected");
            }
        }
    }

    fn apply_panel_visibility_from_config(&mut self) {
        if let Ok(config) = self.config.try_lock() {
            self.bottom_panel.visible = config.panels.bottom;
            self.toolkit_panel.visible = config.panels.toolkit;
            self.swipe_panel.visible = config.panels.swipe;
        }
    }

    fn apply_theme(&self, ctx: &egui::Context) {
        if let Ok(config) = self.config.try_lock() {
            match config.theme.as_str() {
                "dark" => ctx.set_visuals(egui::Visuals::dark()),
                "light" => ctx.set_visuals(egui::Visuals::light()),
                _ => ctx.set_visuals(egui::Visuals::default()),
            }
        }
    }

    fn show_control_panel(&mut self, ui: &mut Ui) {
        ui.heading("Control Panel");

        if let Some(device) = self.device_list.selected_device() {
            ui.group(|ui| {
                ui.label(format!("Selected Device: {}", device.model));
                ui.label(format!("ID: {}", device.identifier));
                ui.label(format!("Status: {:?}", device.status));
            });
        } else {
            ui.label(RichText::new("No device selected").color(Color32::GRAY));
        }

        ui.separator();

        ui.group(|ui| {
            ui.heading("Scrcpy Controls");

            let mut start_scrcpy = false;
            let mut stop_scrcpy = false;

            ui.horizontal(|ui| {
                if ui.button("▶ Start Scrcpy").clicked() {
                    start_scrcpy = true;
                }

                if ui.button("■ Stop Scrcpy").clicked() {
                    stop_scrcpy = true;
                }
            });

            if start_scrcpy {
                self.start_scrcpy();
            }
            if stop_scrcpy {
                self.stop_scrcpy();
            }

            // Quick settings
            ui.label("Quick Settings:");

            let mut config = self.config.try_lock().unwrap();

            ui.horizontal(|ui| {
                ui.label("Bitrate:");
                ui.add(egui::Slider::new(&mut config.bitrate, 1000..=20000).text("KB/s"));
            });

            ui.horizontal(|ui| {
                ui.checkbox(&mut config.show_touches, "Show touches");
                ui.checkbox(&mut config.fullscreen, "Fullscreen");
                ui.checkbox(&mut config.turn_screen_off, "Turn screen off");
            });

            // Max dimensions from settings (adjustable)
            ui.horizontal(|ui| {
                let mut dim_val = config.dimension.unwrap_or(0);
                ui.label("Max dimensions:");
                if ui.add(egui::DragValue::new(&mut dim_val).clamp_range(0..=8192).speed(10)).changed() {
                    if dim_val == 0 {
                        config.dimension = None;
                    } else {
                        config.dimension = Some(dim_val);
                    }
                }
                if ui.button("Unlimited").clicked() {
                    config.dimension = None;
                }
                if let Some(dim) = config.dimension {
                    ui.label(format!("({} px)", dim));
                } else {
                    ui.label("(unlimited)");
                }
            });
        });

        if let Ok(config) = self.config.try_lock() {
            if config.panels.swipe {
                ui.separator();
                self.swipe_panel.show(ui);
            }
        }
    }

    fn start_scrcpy(&mut self) {
        if self.debug_disable_scrcpy {
            self.status_message = "Scrcpy is disabled in debug mode".to_string();
            return;
        }

        if let (Some(scrcpy_bridge), Some(device)) =
            (&self.scrcpy_bridge, self.device_list.selected_device())
        {
            let config = self.config.try_lock().unwrap();

            // Log configuration details
            info!("Starting scrcpy with configuration:");
            info!("  Device: {} ({})", device.model, device.identifier);
            info!("  Bitrate: {} KB/s", config.bitrate);
            info!("  Orientation: {:?}", config.orientation);
            info!("  Show touches: {}", config.show_touches);
            info!("  Display force on: {}", config.turn_screen_off);
            info!("  Fullscreen: {}", config.fullscreen);
            info!("  Dimension: {:?}", config.dimension);
            info!("  Extra args: '{}'", config.extra_args);

            let args = scrcpy_bridge.build_args(
                Some(&device.identifier),
                config.bitrate,
                config.orientation.clone(),
                config.show_touches,
                config.fullscreen,
                config.dimension,
                &config.extra_args,
                config.turn_screen_off,
            );

            info!("Built scrcpy arguments: {:?}", args);
            info!("Scrcpy path: {}", scrcpy_bridge.path());

            match scrcpy_bridge.start(&args) {
                Ok(_child) => {
                    info!("Scrcpy started successfully");
                    self.status_message = "Scrcpy started".to_string();
                }
                Err(e) => {
                    error!("Failed to start scrcpy: {}", e);
                    self.status_message = format!("Failed to start scrcpy: {}", e);
                }
            }
        } else {
            self.status_message = "No device selected or scrcpy not configured".to_string();
        }
    }

    fn stop_scrcpy(&mut self) {
        use std::process::Command;

        #[cfg(target_os = "windows")]
        {
            let _ = Command::new("taskkill")
                .args(["/F", "/IM", "scrcpy.exe"])
                .output();
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = Command::new("pkill").arg("scrcpy").output();
        }

        self.status_message = "Scrcpy stopped".to_string();
    }

    fn handle_wireless_adb_action(&mut self, action: crate::ui::panels::WirelessAdbAction) {
        use crate::ui::panels::WirelessAdbAction;

        if let Some(adb_bridge) = &self.adb_bridge {
            match action {
                WirelessAdbAction::Connect { ip, port } => match adb_bridge.connect(&ip, port) {
                    Ok(()) => {
                        info!("Successfully connected to {}:{}", ip, port);
                        self.status_message = format!("Connected to {}:{}", ip, port);
                        self.refresh_devices();
                    }
                    Err(e) => {
                        error!("Failed to connect to {}:{}: {}", ip, port, e);
                        self.status_message = format!("Connection failed: {}", e);
                    }
                },
                WirelessAdbAction::EnableTcpip { device_id, port } => {
                    match adb_bridge.tcpip(port, Some(&device_id)) {
                        Ok(()) => {
                            info!("Enabled TCP/IP on device {}:{}", device_id, port);
                            self.status_message =
                                format!("TCP/IP enabled on {}:{}", device_id, port);
                        }
                        Err(e) => {
                            error!(
                                "Failed to enable TCP/IP on device {}:{}: {}",
                                device_id, port, e
                            );
                            self.status_message = format!("TCP/IP enable failed: {}", e);
                        }
                    }
                }
                WirelessAdbAction::Pair { ip, port, code } => {
                    match adb_bridge.pair(&ip, port, &code) {
                        Ok(()) => {
                            info!("Successfully paired with {}:{}", ip, port);
                            self.status_message = format!("Paired with {}:{}", ip, port);
                            self.refresh_devices();
                        }
                        Err(e) => {
                            error!("Failed to pair with {}:{}: {}", ip, port, e);
                            self.status_message = format!("Pairing failed: {}", e);
                        }
                    }
                }
            }
        } else {
            self.status_message = "ADB not configured".to_string();
        }
    }

    fn handle_toolkit_action(&mut self, action: crate::ui::panels::ToolkitAction) {
        use crate::ui::panels::ToolkitAction;
        if let (Some(adb_bridge), Some(device)) =
            (&self.adb_bridge, self.device_list.selected_device())
        {
            match action {
                ToolkitAction::Screenshot => {
                    // Save screenshot to desktop
                    let desktop = dirs::desktop_dir().unwrap_or_default();
                    let file_path = desktop.join("screenshot.png");
                    let status = std::process::Command::new(adb_bridge.path())
                        .args(["-s", &device.identifier, "exec-out", "screencap", "-p"])
                        .stdout(std::fs::File::create(&file_path).unwrap())
                        .status();
                    match status {
                        Ok(s) if s.success() => {
                            self.status_message =
                                format!("Screenshot saved to {}", file_path.display());
                        }
                        Ok(s) => {
                            self.status_message = format!("Screenshot failed: exit code {}", s);
                        }
                        Err(e) => {
                            self.status_message = format!("Screenshot error: {}", e);
                        }
                    }
                }
                ToolkitAction::RecordScreen => {
                    // Start screenrecord (fixed 10s for demo)
                    let status = std::process::Command::new(adb_bridge.path())
                        .args([
                            "-s",
                            &device.identifier,
                            "shell",
                            "screenrecord",
                            "/sdcard/video.mp4",
                            "--time-limit",
                            "10",
                        ])
                        .status();
                    match status {
                        Ok(s) if s.success() => {
                            // Pull the file
                            let desktop = dirs::desktop_dir().unwrap_or_default();
                            let file_path = desktop.join("video.mp4");
                            let pull_status = std::process::Command::new(adb_bridge.path())
                                .args([
                                    "-s",
                                    &device.identifier,
                                    "pull",
                                    "/sdcard/video.mp4",
                                    file_path.to_str().unwrap(),
                                ])
                                .status();
                            match pull_status {
                                Ok(ps) if ps.success() => {
                                    self.status_message =
                                        format!("Screenrecord saved to {}", file_path.display());
                                }
                                Ok(ps) => {
                                    self.status_message = format!("Pull failed: exit code {}", ps);
                                }
                                Err(e) => {
                                    self.status_message = format!("Pull error: {}", e);
                                }
                            }
                        }
                        Ok(s) => {
                            self.status_message = format!("Screenrecord failed: exit code {}", s);
                        }
                        Err(e) => {
                            self.status_message = format!("Screenrecord error: {}", e);
                        }
                    }
                }
                ToolkitAction::InstallApk => {
                    // Open file picker (native dialog)
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("APK", &["apk"])
                        .pick_file()
                    {
                        let status = std::process::Command::new(adb_bridge.path())
                            .args(["-s", &device.identifier, "install", path.to_str().unwrap()])
                            .status();
                        match status {
                            Ok(s) if s.success() => {
                                self.status_message = format!("Installed APK: {}", path.display());
                            }
                            Ok(s) => {
                                self.status_message = format!("Install failed: exit code {}", s);
                            }
                            Err(e) => {
                                self.status_message = format!("Install error: {}", e);
                            }
                        }
                    }
                }
                ToolkitAction::FileManager => {
                    // Open device shell directly in terminal (macOS approach)
                    let adb_path = adb_bridge.path();
                    let device_id = &device.identifier;

                    // Open Terminal with ADB shell command directly
                    let _ = std::process::Command::new("open")
                        .arg("-a")
                        .arg("Terminal")
                        .arg("-e")
                        .arg(adb_path)
                        .arg("-s")
                        .arg(device_id)
                        .arg("shell")
                        .spawn();

                    self.status_message = "Opened device shell in terminal".to_string();
                }
                ToolkitAction::OpenShell => {
                    // Open ADB shell directly in terminal (macOS approach)
                    let adb_path = adb_bridge.path();
                    let device_id = &device.identifier;

                    // Open Terminal with ADB shell command directly
                    let _ = std::process::Command::new("open")
                        .arg("-a")
                        .arg("Terminal")
                        .arg("-e")
                        .arg(adb_path)
                        .arg("-s")
                        .arg(device_id)
                        .arg("shell")
                        .spawn();

                    self.status_message = "Opened ADB shell in terminal".to_string();
                }
                ToolkitAction::None => {}
            }
        } else if let ToolkitAction::None = action {
            // do nothing
        } else {
            self.status_message = "No device selected or ADB not configured".to_string();
        }
    }
}

impl eframe::App for DroidViewApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.settings_window.take_just_saved() {
            self.update_bridges();
            self.refresh_devices();
            self.status_message = "Settings saved and applied.".to_string();
            self.apply_panel_visibility_from_config();
            self.apply_theme(ctx);
        }
        self.update_bridges();
        self.refresh_devices();
        self.update_scrcpy_status();

        // Left panel (device list)
        egui::SidePanel::left("device_panel")
            .resizable(true)
            .default_width(250.0)
            .show(ctx, |ui| {
                self.device_list.show(ui);
                // Status bar below device list
                ui.separator();
                let status_color = if self.scrcpy_running {
                    Color32::GREEN
                } else {
                    Color32::GRAY
                };
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&self.status_message).color(status_color));
                    if self.scrcpy_running {
                        ui.label(RichText::new("🟢 scrcpy running").color(Color32::GREEN));
                    } else {
                        ui.label(RichText::new("🔴 scrcpy stopped").color(Color32::RED));
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("🔄 Refresh").clicked() {
                        self.refresh_devices();
                    }
                    if ui.button("🔄 Restart ADB").clicked() {
                        if let Some(adb_bridge) = &self.adb_bridge {
                            if let Err(e) = crate::device::restart_adb_server(adb_bridge.path()) {
                                error!("Failed to restart ADB: {}", e);
                                self.status_message = format!("ADB restart failed: {}", e);
                            } else {
                                self.status_message = "ADB restarted".to_string();
                                self.refresh_devices();
                            }
                        }
                    }
                });
                ui.separator();
                if let Some(action) = self.wireless_adb_panel.show(ui, self.adb_bridge.as_ref(), &self.devices) {
                    self.handle_wireless_adb_action(action);
                }
            });

        // Right panel (toolkit)
        let available_width = ctx.available_rect().width();
        let right_panel_default_width = available_width * 0.3;
        let right_panel_width = right_panel_default_width.max(300.0);
        if self.toolkit_panel.visible {
            egui::SidePanel::right("toolkit_panel")
                .resizable(true)
                .default_width(right_panel_width)
                .min_width(180.0)
                .show(ctx, |ui| {
                    let toolkit_action = self.toolkit_panel.show(ui);
                    self.handle_toolkit_action(toolkit_action);
                });
        }

        // Central panel (main content)
        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_control_panel(ui);
            if self.bottom_panel.visible {
                egui::TopBottomPanel::bottom("bottom_panel")
                    .resizable(true)
                    .default_height(100.0)
                    .show_inside(ui, |ui| {
                        let action = self.bottom_panel.show(ui);
                        use crate::ui::panels::BottomPanelAction;
                        match action {
                            BottomPanelAction::RefreshDevices => self.refresh_devices(),
                            BottomPanelAction::RestartAdb => {
                                if let Some(adb_bridge) = &self.adb_bridge {
                                    if let Err(e) = crate::device::restart_adb_server(adb_bridge.path()) {
                                        error!("Failed to restart ADB: {}", e);
                                        self.status_message = format!("ADB restart failed: {}", e);
                                    } else {
                                        self.status_message = "ADB restarted".to_string();
                                        self.refresh_devices();
                                    }
                                }
                            }
                            BottomPanelAction::OpenSettings => self.settings_window.open(),
                            BottomPanelAction::None => {}
                        }
                    });
            }
        });

        self.settings_window.show(ctx);
    }
}
