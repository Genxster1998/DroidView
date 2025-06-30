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
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{error, info};
use crate::utils::is_process_running;
use crate::ui::BottomPanelAction;
use std::collections::HashMap;
use egui_knob::{Knob, KnobStyle, LabelPosition};

// Background task results
#[derive(Debug)]
enum BackgroundTaskResult {
    AppList(Vec<(String, String)>),
    DisableAppList(Vec<(String, String)>),
    Imei(String),
    DisplayInfo(String),
    BatteryInfo(String),
}

// Wrapper types for different task results
pub struct AppListResult(pub Vec<(String, String)>);
pub struct DisableAppListResult(pub Vec<(String, String)>);
pub struct ImeiResult(pub String);
pub struct BatteryInfoResult(pub String);

impl From<AppListResult> for BackgroundTaskResult {
    fn from(result: AppListResult) -> Self {
        BackgroundTaskResult::AppList(result.0)
    }
}

impl From<DisableAppListResult> for BackgroundTaskResult {
    fn from(result: DisableAppListResult) -> Self {
        BackgroundTaskResult::DisableAppList(result.0)
    }
}

impl From<ImeiResult> for BackgroundTaskResult {
    fn from(result: ImeiResult) -> Self {
        BackgroundTaskResult::Imei(result.0)
    }
}

impl From<BatteryInfoResult> for BackgroundTaskResult {
    fn from(result: BatteryInfoResult) -> Self {
        BackgroundTaskResult::BatteryInfo(result.0)
    }
}

impl From<Vec<(String, String)>> for BackgroundTaskResult {
    fn from(apps: Vec<(String, String)>) -> Self {
        BackgroundTaskResult::AppList(apps)
    }
}

impl From<String> for BackgroundTaskResult {
    fn from(info: String) -> Self {
        BackgroundTaskResult::DisplayInfo(info)
    }
}

// Embed the icon at compile time
pub const ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

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
    imei_popup: Option<String>,
    display_popup: Option<String>,
    battery_popup: Option<String>,
    screenrecord_dialog: bool,
    screenrecord_duration: u32,
    screenrecord_bitrate: u32,
    uninstall_dialog: bool,
    app_list: Vec<(String, String)>, // (package_name, app_name)
    selected_apps: std::collections::HashSet<String>, // package names
    disable_dialog: bool,
    disable_app_list: Vec<(String, String)>, // (package_name, app_name)
    selected_disable_apps: std::collections::HashSet<String>, // package names
    about_dialog: bool,
    // Async processing states
    loading_apps: bool,
    loading_disable_apps: bool,
    loading_imei: bool,
    loading_display_info: bool,
    loading_battery_info: bool,
    // Background task management
    task_handles: HashMap<String, JoinHandle<()>>,
    result_receiver: mpsc::UnboundedReceiver<BackgroundTaskResult>,
    result_sender: mpsc::UnboundedSender<BackgroundTaskResult>,
}

impl DroidViewApp {
    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        config: Arc<Mutex<AppConfig>>,
        debug_disable_scrcpy: bool,
    ) -> Self {
        let (result_sender, result_receiver) = mpsc::unbounded_channel();
        
        let mut app = Self {
            config: config.clone(),
            devices: Vec::new(),
            device_list: DeviceList::new(),
            swipe_panel: SwipePanel::new(),
            toolkit_panel: ToolkitPanel::new(),
            bottom_panel: BottomPanel::new(),
            wireless_adb_panel: WirelessAdbPanel::new(),
            settings_window: SettingsWindow::new(config.clone()),
            adb_bridge: None,
            scrcpy_bridge: None,
            status_message: String::new(),
            scrcpy_running: false,
            debug_disable_scrcpy,
            imei_popup: None,
            display_popup: None,
            battery_popup: None,
            screenrecord_dialog: false,
            screenrecord_duration: 10,
            screenrecord_bitrate: 8000000,
            uninstall_dialog: false,
            app_list: Vec::new(),
            selected_apps: std::collections::HashSet::new(),
            disable_dialog: false,
            disable_app_list: Vec::new(),
            selected_disable_apps: std::collections::HashSet::new(),
            about_dialog: false,
            // Async processing states
            loading_apps: false,
            loading_disable_apps: false,
            loading_imei: false,
            loading_display_info: false,
            loading_battery_info: false,
            // Background task management
            task_handles: HashMap::new(),
            result_receiver,
            result_sender,
        };
        
        // Set config for wireless ADB panel to remember IPs
        app.wireless_adb_panel.set_config(config);
        
        app
    }

    fn update_bridges(&mut self) {
        let mut config = self.config.try_lock().unwrap();

        // Auto-detect adb if not configured
        if config.adb_path.is_none() {
            if let Some(adb_path) = crate::utils::find_adb() {
                config.adb_path = Some(adb_path.display().to_string());
                info!(
                    "Auto-detected ADB at: {}",
                    config.adb_path.as_ref().unwrap()
                );
            }
        }

        // Auto-detect scrcpy if not configured
        if config.scrcpy_path.is_none() {
            if let Some(scrcpy_path) = crate::utils::find_scrcpy() {
                config.scrcpy_path = Some(scrcpy_path.display().to_string());
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

    fn run_background_task<F, T>(&mut self, task_id: String, task: F) 
    where
        F: FnOnce() -> T + Send + 'static,
        T: Into<BackgroundTaskResult> + Send + 'static,
    {
        let sender = self.result_sender.clone();
        
        let handle = tokio::task::spawn_blocking(move || {
            let result = task();
            let _ = sender.send(result.into());
        });
        
        self.task_handles.insert(task_id, handle);
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

            // --- Bitrate knob and quick settings ---
            {
                let mut config = self.config.try_lock().unwrap();
                // Bitrate adjustment knob
                let mut bitrate_value: u32 = {
                    let s = config.bitrate.trim().to_uppercase();
                    if s.ends_with('M') {
                        s.trim_end_matches('M').parse::<u32>().unwrap_or(8) * 1000
                    } else if s.ends_with('K') {
                        s.trim_end_matches('K').parse::<u32>().unwrap_or(8000)
                    } else {
                        s.parse::<u32>().unwrap_or(8000)
                    }
                };
                let mut bitrate_unit = if config.bitrate.trim().to_uppercase().ends_with('M') {
                    "Mbps"
                } else {
                    "Kbps"
                };
                ui.vertical(|ui| {
                    let mut knob_value = bitrate_value as f32;
                    let knob = Knob::new(&mut knob_value, 100.0, 20000.0, KnobStyle::Dot)
                        .with_size(60.0)
                        .with_font_size(14.0)
                        .with_stroke_width(3.0)
                        .with_colors(Color32::GRAY, Color32::WHITE, Color32::WHITE)
                        .with_label("Bitrate", LabelPosition::Top);
                    let knob_resp = ui.add(knob);
                    if knob_resp.changed() {
                        knob_value = (knob_value / 100.0).round() * 100.0;
                        bitrate_value = knob_value as u32;
                    }
                    egui::ComboBox::new("scrcpy_bitrate_unit_combo", "Unit")
                        .selected_text(bitrate_unit)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut bitrate_unit, "Kbps", "Kbps");
                            ui.selectable_value(&mut bitrate_unit, "Mbps", "Mbps");
                        });
                    let bitrate_str = if bitrate_unit == "Mbps" {
                        format!("{}M", (bitrate_value as f32 / 1000.0).round() as u32)
                    } else {
                        format!("{}K", bitrate_value)
                    };
                    config.bitrate = bitrate_str;
                    ui.label(format!("Current: {}", config.bitrate));
                });

                // Quick settings
                ui.label("Quick Settings:");
                ui.horizontal(|ui| {
                    ui.checkbox(&mut config.show_touches, "Show touches");
                    ui.checkbox(&mut config.fullscreen, "Fullscreen");
                    ui.checkbox(&mut config.turn_screen_off, "Turn screen off");
                });

                // Max dimensions from settings (adjustable)
                ui.horizontal(|ui| {
                    let mut dim_val = config.dimension.unwrap_or(0);
                    ui.label("Max dimensions:");
                    if ui.add(egui::DragValue::new(&mut dim_val).range(0..=8192).speed(10)).changed() {
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
            }
            // --- End config lock scope ---

            if start_scrcpy {
                self.start_scrcpy();
            }
            if stop_scrcpy {
                self.stop_scrcpy();
            }
        });

        if let Ok(config) = self.config.try_lock() {
            if config.panels.swipe {
                ui.separator();
                if let Some(swipe_action) = self.swipe_panel.show(ui) {
                    if let (Some(adb_bridge), Some(device)) = (self.adb_bridge.as_ref(), self.device_list.selected_device()) {
                        // Get screen size
                        let output = std::process::Command::new(adb_bridge.path())
                            .args(["-s", &device.identifier, "shell", "wm size"])
                            .output();
                        if let Ok(output) = output {
                            if output.status.success() {
                                let out = String::from_utf8_lossy(&output.stdout);
                                if let Some(size_str) = out.split_whitespace().find(|s| s.contains('x')) {
                                    let parts: Vec<&str> = size_str.split('x').collect();
                                    if parts.len() == 2 {
                                        if let (Ok(width), Ok(height)) = (parts[0].parse::<i32>(), parts[1].parse::<i32>()) {
                                            // Calculate swipe coordinates
                                            let (x1, y1, x2, y2) = match swipe_action {
                                                crate::ui::panels::SwipeAction::Up => (width/2, (height*4)/5, width/2, height/5),
                                                crate::ui::panels::SwipeAction::Down => (width/2, height/5, width/2, (height*4)/5),
                                                crate::ui::panels::SwipeAction::Left => ((width*4)/5, height/2, width/5, height/2),
                                                crate::ui::panels::SwipeAction::Right => (width/5, height/2, (width*4)/5, height/2),
                                            };
                                            let swipe_cmd = format!("input swipe {} {} {} {} 300", x1, y1, x2, y2);
                                            let swipe_out = std::process::Command::new(adb_bridge.path())
                                                .args(["-s", &device.identifier, "shell", &swipe_cmd])
                                                .output();
                                            if let Ok(swipe_out) = swipe_out {
                                                if swipe_out.status.success() {
                                                    self.status_message = "Swipe sent successfully".to_string();
                                                } else {
                                                    self.status_message = "Swipe command failed".to_string();
                                                }
                                            } else {
                                                self.status_message = "Failed to send swipe command".to_string();
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        self.status_message = "No device selected or ADB not configured".to_string();
                    }
                }
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
            info!("  Bitrate: {}", config.bitrate);
            info!("  Orientation: {:?}", config.orientation);
            info!("  Show touches: {}", config.show_touches);
            info!("  Display force on: {}", config.turn_screen_off);
            info!("  Fullscreen: {}", config.fullscreen);
            info!("  Dimension: {:?}", config.dimension);
            info!("  Extra args: '{}'", config.extra_args);

            let args = scrcpy_bridge.build_args(
                Some(&device.identifier),
                &config.bitrate,
                config.orientation.clone(),
                config.show_touches,
                config.fullscreen,
                config.dimension,
                &config.extra_args,
                config.turn_screen_off,
                config.force_adb_forward,
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
                    // Show screen recording dialog
                    self.screenrecord_dialog = true;
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
                ToolkitAction::OpenShell => {
                    // Open ADB shell directly in terminal (cross-platform)
                    let adb_path = adb_bridge.path();
                    let device_id = &device.identifier;

                    #[cfg(target_os = "macos")]
                    {
                        // Use osascript to open Terminal with ADB shell command
                        let script = format!(
                            "tell application \"Terminal\" to do script \"{} -s {} shell\"",
                            adb_path, device_id
                        );
                        
                        let _ = std::process::Command::new("osascript")
                            .arg("-e")
                            .arg(script)
                            .spawn();
                    }

                    #[cfg(target_os = "windows")]
                    {
                        // Use cmd to open Command Prompt with ADB shell command
                        let _ = std::process::Command::new("cmd")
                            .args(["/C", "start", "cmd", "/K", &format!("{} -s {} shell", adb_path, device_id)])
                            .spawn();
                    }

                    #[cfg(target_os = "linux")]
                    {
                        // Try different terminal emulators on Linux
                        let terminals: &[(&str, &[&str])] = &[
                            ("gnome-terminal", &["--", "bash", "-c", &format!("{} -s {} shell; exec bash", adb_path, device_id)]),
                            ("konsole", &["-e", "bash", "-c", &format!("{} -s {} shell; exec bash", adb_path, device_id)]),
                            ("xterm", &["-e", "bash", "-c", &format!("{} -s {} shell; exec bash", adb_path, device_id)]),
                            ("terminator", &["-e", &format!("{} -s {} shell", adb_path, device_id)]),
                            ("xfce4-terminal", &["-e", &format!("{} -s {} shell", adb_path, device_id)]),
                        ];

                        let mut opened = false;
                        for (terminal, args) in terminals {
                            if std::process::Command::new(terminal).args(*args).spawn().is_ok() {
                                opened = true;
                                break;
                            }
                        }

                        if !opened {
                            // Fallback: try to open default terminal
                            let _ = std::process::Command::new("x-terminal-emulator")
                                .arg("-e")
                                .arg(format!("{} -s {} shell", adb_path, device_id))
                                .spawn();
                        }
                    }

                    self.status_message = "Opened ADB shell in terminal".to_string();
                }
                ToolkitAction::ShowImei => {
                    // Start async IMEI fetching if not already loading
                    if !self.loading_imei && !self.task_handles.contains_key("imei") {
                        if let (Some(adb_bridge), Some(device)) = (self.adb_bridge.as_ref(), self.device_list.selected_device()) {
                            self.loading_imei = true;
                            let adb_path = adb_bridge.path().to_string();
                            let device_id = device.identifier.clone();
                            
                            // Spawn background task
                            self.run_background_task("imei".to_string(), move || {
                                // Try multiple IMEI retrieval methods for different Android versions and dual-SIM devices
                                let mut imei_result = String::new();
                                
                                // Method 1: For Android 10+ (requires READ_PHONE_STATE permission)
                                let output1 = std::process::Command::new(&adb_path)
                                    .args([
                                        "-s",
                                        &device_id,
                                        "shell",
                                        "settings get secure android_id"
                                    ])
                                    .output();
                                
                                if let Ok(output) = output1 {
                                    if output.status.success() {
                                        let android_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
                                        if !android_id.is_empty() {
                                            imei_result.push_str(&format!("Android ID: {}\n", android_id));
                                        }
                                    }
                                }
                                
                                // Method 2: For dual-SIM devices (Android 5+)
                                let output2 = std::process::Command::new(&adb_path)
                                    .args([
                                        "-s",
                                        &device_id,
                                        "shell",
                                        "getprop ro.telephony.imei"
                                    ])
                                    .output();
                                
                                if let Ok(output) = output2 {
                                    if output.status.success() {
                                        let imei = String::from_utf8_lossy(&output.stdout).trim().to_string();
                                        if !imei.is_empty() && imei != "0" {
                                            imei_result.push_str(&format!("IMEI: {}\n", imei));
                                        }
                                    }
                                }
                                
                                // Method 3: For dual-SIM devices - IMEI1 and IMEI2
                                let output3 = std::process::Command::new(&adb_path)
                                    .args([
                                        "-s",
                                        &device_id,
                                        "shell",
                                        "getprop ro.telephony.imei1"
                                    ])
                                    .output();
                                
                                if let Ok(output) = output3 {
                                    if output.status.success() {
                                        let imei1 = String::from_utf8_lossy(&output.stdout).trim().to_string();
                                        if !imei1.is_empty() && imei1 != "0" {
                                            imei_result.push_str(&format!("IMEI1: {}\n", imei1));
                                        }
                                    }
                                }
                                
                                let output4 = std::process::Command::new(&adb_path)
                                    .args([
                                        "-s",
                                        &device_id,
                                        "shell",
                                        "getprop ro.telephony.imei2"
                                    ])
                                    .output();
                                
                                if let Ok(output) = output4 {
                                    if output.status.success() {
                                        let imei2 = String::from_utf8_lossy(&output.stdout).trim().to_string();
                                        if !imei2.is_empty() && imei2 != "0" {
                                            imei_result.push_str(&format!("IMEI2: {}\n", imei2));
                                        }
                                    }
                                }
                                
                                // Method 4: Legacy method for older devices (deprecated but might work on some)
                                let output5 = std::process::Command::new(&adb_path)
                                    .args([
                                        "-s",
                                        &device_id,
                                        "shell",
                                        "service call iphonesubinfo 4 | cut -c 52-66 | tr -d '.[:space:]'"
                                    ])
                                    .output();
                                
                                if let Ok(output) = output5 {
                                    if output.status.success() {
                                        let imei = String::from_utf8_lossy(&output.stdout).trim().to_string();
                                        if !imei.is_empty() && imei.len() >= 14 {
                                            imei_result.push_str(&format!("Legacy IMEI: {}\n", imei));
                                        }
                                    }
                                }
                                
                                // Method 5: Get device serial number as fallback
                                let output6 = std::process::Command::new(&adb_path)
                                    .args([
                                        "-s",
                                        &device_id,
                                        "shell",
                                        "getprop ro.serialno"
                                    ])
                                    .output();
                                
                                if let Ok(output) = output6 {
                                    if output.status.success() {
                                        let serial = String::from_utf8_lossy(&output.stdout).trim().to_string();
                                        if !serial.is_empty() {
                                            imei_result.push_str(&format!("Serial: {}\n", serial));
                                        }
                                    }
                                }
                                
                                if !imei_result.is_empty() {
                                    ImeiResult(imei_result.trim().to_string())
                                } else {
                                    ImeiResult("No IMEI/Device ID information available. This may be due to:\n• Android security restrictions (Android 10+)\n• Missing READ_PHONE_STATE permission\n• Device not supporting IMEI retrieval".to_string())
                                }
                            });
                            
                            self.status_message = "Loading IMEI...".to_string();
                        } else {
                            self.status_message = "No device selected or ADB not configured".to_string();
                        }
                    }
                }
                ToolkitAction::DisplayInfo => {
                    // Start async display info fetching if not already loading
                    if !self.loading_display_info && !self.task_handles.contains_key("display_info") {
                        if let (Some(adb_bridge), Some(device)) = (self.adb_bridge.as_ref(), self.device_list.selected_device()) {
                            self.loading_display_info = true;
                            let adb_path = adb_bridge.path().to_string();
                            let device_id = device.identifier.clone();
                            
                            // Spawn background task
                            self.run_background_task("display_info".to_string(), move || {
                                let mut display_info = String::new();
                                
                                // Get dumpsys display info
                                let dumpsys_output = std::process::Command::new(&adb_path)
                                    .args([
                                        "-s",
                                        &device_id,
                                        "shell",
                                        "dumpsys display | grep -E 'Flags|Display.*:|location'"
                                    ])
                                    .output();

                                if let Ok(output) = dumpsys_output {
                                    if output.status.success() {
                                        display_info.push_str("📱 Display Information:\n");
                                        display_info.push_str(&String::from_utf8_lossy(&output.stdout));
                                        display_info.push_str("\n\n");
                                    }
                                }

                                // Get wm size info
                                let wm_size_output = std::process::Command::new(&adb_path)
                                    .args([
                                        "-s",
                                        &device_id,
                                        "shell",
                                        "wm size"
                                    ])
                                    .output();

                                if let Ok(output) = wm_size_output {
                                    if output.status.success() {
                                        display_info.push_str("📐 Window Manager Size:\n");
                                        display_info.push_str(&String::from_utf8_lossy(&output.stdout));
                                        display_info.push_str("\n\n");
                                    }
                                }

                                // Get wm density info
                                let wm_density_output = std::process::Command::new(&adb_path)
                                    .args([
                                        "-s",
                                        &device_id,
                                        "shell",
                                        "wm density"
                                    ])
                                    .output();

                                if let Ok(output) = wm_density_output {
                                    if output.status.success() {
                                        display_info.push_str("📊 Window Manager Density:\n");
                                        display_info.push_str(&String::from_utf8_lossy(&output.stdout));
                                    }
                                }

                                if !display_info.is_empty() {
                                    display_info
                                } else {
                                    "Failed to retrieve display info".to_string()
                                }
                            });
                            
                            self.status_message = "Loading display info...".to_string();
                        } else {
                            self.status_message = "No device selected or ADB not configured".to_string();
                        }
                    }
                }
                ToolkitAction::BatteryInfo => {
                    // Start async battery info fetching if not already loading
                    if !self.loading_battery_info && !self.task_handles.contains_key("battery_info") {
                        if let (Some(adb_bridge), Some(device)) = (self.adb_bridge.as_ref(), self.device_list.selected_device()) {
                            self.loading_battery_info = true;
                            let adb_path = adb_bridge.path().to_string();
                            let device_id = device.identifier.clone();
                            
                            // Spawn background task
                            self.run_background_task("battery_info".to_string(), move || {
                                let output = std::process::Command::new(&adb_path)
                                    .args([
                                        "-s",
                                        &device_id,
                                        "shell",
                                        "dumpsys battery"
                                    ])
                                    .output();

                                match output {
                                    Ok(output) if output.status.success() => {
                                        let output_str = String::from_utf8_lossy(&output.stdout);
                                        BatteryInfoResult(output_str.to_string())
                                    }
                                    _ => BatteryInfoResult("Failed to retrieve battery info".to_string()),
                                }
                            });
                            
                            self.status_message = "Loading battery info...".to_string();
                        } else {
                            self.status_message = "No device selected or ADB not configured".to_string();
                        }
                    }
                }
                ToolkitAction::UninstallApp => {
                    // Start async app list fetching if not already loading
                    if !self.loading_apps && !self.task_handles.contains_key("app_list") {
                        if let (Some(adb_bridge), Some(device)) = (self.adb_bridge.as_ref(), self.device_list.selected_device()) {
                            self.loading_apps = true;
                            let adb_path = adb_bridge.path().to_string();
                            let device_id = device.identifier.clone();
                            
                            // Spawn background task
                            self.run_background_task("app_list".to_string(), move || {
                                let output = std::process::Command::new(&adb_path)
                                    .args([
                                        "-s",
                                        &device_id,
                                        "shell",
                                        "pm list packages -3"
                                    ])
                                    .output();

                                match output {
                                    Ok(output) if output.status.success() => {
                                        let mut apps = Vec::new();
                                        for line in String::from_utf8_lossy(&output.stdout).lines() {
                                            if line.starts_with("package:") {
                                                let package_name = line.replace("package:", "").trim().to_string();
                                                apps.push((package_name.clone(), package_name));
                                            }
                                        }
                                        AppListResult(apps)
                                    }
                                    _ => AppListResult(Vec::new()),
                                }
                            });
                            
                            self.status_message = "Loading app list...".to_string();
                        } else {
                            self.status_message = "No device selected or ADB not configured".to_string();
                        }
                    }
                }
                ToolkitAction::DisableApp => {
                    // Start async disable app list fetching if not already loading
                    if !self.loading_disable_apps && !self.task_handles.contains_key("disable_app_list") {
                        if let (Some(adb_bridge), Some(device)) = (self.adb_bridge.as_ref(), self.device_list.selected_device()) {
                            self.loading_disable_apps = true;
                            let adb_path = adb_bridge.path().to_string();
                            let device_id = device.identifier.clone();
                            
                            // Spawn background task
                            self.run_background_task("disable_app_list".to_string(), move || {
                                let output = std::process::Command::new(&adb_path)
                                    .args([
                                        "-s",
                                        &device_id,
                                        "shell",
                                        "pm list packages -e"
                                    ])
                                    .output();

                                match output {
                                    Ok(output) if output.status.success() => {
                                        let mut apps = Vec::new();
                                        for line in String::from_utf8_lossy(&output.stdout).lines() {
                                            if line.starts_with("package:") {
                                                let package_name = line.replace("package:", "").trim().to_string();
                                                apps.push((package_name.clone(), package_name));
                                            }
                                        }
                                        DisableAppListResult(apps)
                                    }
                                    _ => DisableAppListResult(Vec::new()),
                                }
                            });
                            
                            self.status_message = "Loading app list...".to_string();
                        } else {
                            self.status_message = "No device selected or ADB not configured".to_string();
                        }
                    }
                }
                ToolkitAction::Reboot => {
                    if let (Some(adb_bridge), Some(device)) = (self.adb_bridge.as_ref(), self.device_list.selected_device()) {
                        let status = std::process::Command::new(adb_bridge.path())
                            .args(["-s", &device.identifier, "reboot"])
                            .status();
                        
                        match status {
                            Ok(s) if s.success() => {
                                self.status_message = "Device reboot initiated".to_string();
                            }
                            Ok(s) => {
                                self.status_message = format!("Reboot failed: exit code {}", s);
                            }
                            Err(e) => {
                                self.status_message = format!("Reboot error: {}", e);
                            }
                        }
                    } else {
                        self.status_message = "No device selected or ADB not configured".to_string();
                    }
                }
                ToolkitAction::Shutdown => {
                    if let (Some(adb_bridge), Some(device)) = (self.adb_bridge.as_ref(), self.device_list.selected_device()) {
                        let status = std::process::Command::new(adb_bridge.path())
                            .args(["-s", &device.identifier, "shell", "reboot", "-p"])
                            .status();
                        
                        match status {
                            Ok(s) if s.success() => {
                                self.status_message = "Device shutdown initiated".to_string();
                            }
                            Ok(s) => {
                                self.status_message = format!("Shutdown failed: exit code {}", s);
                            }
                            Err(e) => {
                                self.status_message = format!("Shutdown error: {}", e);
                            }
                        }
                    } else {
                        self.status_message = "No device selected or ADB not configured".to_string();
                    }
                }
                ToolkitAction::RebootRecovery => {
                    if let (Some(adb_bridge), Some(device)) = (self.adb_bridge.as_ref(), self.device_list.selected_device()) {
                        let status = std::process::Command::new(adb_bridge.path())
                            .args(["-s", &device.identifier, "reboot", "recovery"])
                            .status();
                        
                        match status {
                            Ok(s) if s.success() => {
                                self.status_message = "Device rebooting to recovery mode".to_string();
                            }
                            Ok(s) => {
                                self.status_message = format!("Recovery reboot failed: exit code {}", s);
                            }
                            Err(e) => {
                                self.status_message = format!("Recovery reboot error: {}", e);
                            }
                        }
                    } else {
                        self.status_message = "No device selected or ADB not configured".to_string();
                    }
                }
                ToolkitAction::RebootBootloader => {
                    if let (Some(adb_bridge), Some(device)) = (self.adb_bridge.as_ref(), self.device_list.selected_device()) {
                        let status = std::process::Command::new(adb_bridge.path())
                            .args(["-s", &device.identifier, "reboot", "bootloader"])
                            .status();
                        
                        match status {
                            Ok(s) if s.success() => {
                                self.status_message = "Device rebooting to bootloader".to_string();
                            }
                            Ok(s) => {
                                self.status_message = format!("Bootloader reboot failed: exit code {}", s);
                            }
                            Err(e) => {
                                self.status_message = format!("Bootloader reboot error: {}", e);
                            }
                        }
                    } else {
                        self.status_message = "No device selected or ADB not configured".to_string();
                    }
                }
                ToolkitAction::None => {}
            }
        } else if let ToolkitAction::None = action {
            // do nothing
        } else {
            self.status_message = "No device selected or ADB not configured".to_string();
        }
    }

    fn update_background_tasks(&mut self) {
        // Check for completed tasks
        while let Ok(result) = self.result_receiver.try_recv() {
            match result {
                BackgroundTaskResult::AppList(apps) => {
                    self.loading_apps = false;
                    self.app_list = apps;
                    self.uninstall_dialog = true;
                    self.status_message = "App list loaded successfully".to_string();
                }
                BackgroundTaskResult::DisableAppList(apps) => {
                    self.loading_disable_apps = false;
                    self.disable_app_list = apps;
                    self.disable_dialog = true;
                    self.status_message = "App list loaded successfully".to_string();
                }
                BackgroundTaskResult::Imei(imei) => {
                    self.loading_imei = false;
                    self.imei_popup = Some(imei);
                    self.status_message = "IMEI retrieved successfully".to_string();
                }
                BackgroundTaskResult::DisplayInfo(info) => {
                    self.loading_display_info = false;
                    self.display_popup = Some(info);
                    self.status_message = "Display info retrieved successfully".to_string();
                }
                BackgroundTaskResult::BatteryInfo(info) => {
                    self.loading_battery_info = false;
                    self.battery_popup = Some(info);
                    self.status_message = "Battery info retrieved successfully".to_string();
                }
            }
        }

        // Clean up completed tasks
        self.task_handles.retain(|_, handle| !handle.is_finished());
    }

    fn is_processing(&self) -> bool {
        self.loading_apps || self.loading_disable_apps || self.loading_imei || self.loading_display_info || self.loading_battery_info
    }

    fn toggle_theme(&mut self, ctx: &egui::Context) {
        if let Ok(mut config) = self.config.try_lock() {
            match config.theme.as_str() {
                "dark" => {
                    config.theme = "light".to_string();
                    ctx.set_visuals(egui::Visuals::light());
                }
                "light" => {
                    config.theme = "dark".to_string();
                    ctx.set_visuals(egui::Visuals::dark());
                }
                _ => {
                    config.theme = "dark".to_string();
                    ctx.set_visuals(egui::Visuals::dark());
                }
            }
            // Save the theme change
            let _ = config.save();
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
        let right_panel_width = right_panel_default_width.max(200.0);
        if self.toolkit_panel.visible {
            use crate::ui::panels::ToolkitLoadingState;
            let loading = ToolkitLoadingState {
                screenshot: false,
                record_screen: false,
                install_apk: false,
                open_shell: false,
                show_imei: self.loading_imei,
                display_info: self.loading_display_info,
                battery_info: self.loading_battery_info,
                uninstall_app: self.loading_apps,
                disable_app: self.loading_disable_apps,
            };
            egui::SidePanel::right("toolkit_panel")
                .resizable(true)
                .default_width(right_panel_width)
                .min_width(180.0)
                .show(ctx, |ui| {
                    let toolkit_action = self.toolkit_panel.show(ui, &loading);
                    self.handle_toolkit_action(toolkit_action);
                    
                    // Add processing status at the bottom of the right panel
                    if self.is_processing() {
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.add(egui::Spinner::new().size(16.0));
                            ui.label(egui::RichText::new("Processing...").size(13.0).color(Color32::YELLOW));
                        });
                    }
                    
                    // Theme switch and About button
                    ui.separator();
                    ui.horizontal(|ui| {
                        // Theme toggle button
                        let current_theme = if let Ok(config) = self.config.try_lock() {
                            config.theme.clone()
                        } else {
                            "default".to_string()
                        };
                        
                        let theme_text = match current_theme.as_str() {
                            "dark" => "🌙 Dark",
                            "light" => "☀️ Light",
                            _ => "🌙 Dark"
                        };
                        
                        if ui.button(egui::RichText::new(theme_text).size(12.0)).clicked() {
                            self.toggle_theme(ctx);
                        }
                        
                        ui.separator();
                        
                        // About button
                        if ui.button(egui::RichText::new("ℹ️ About").size(12.0)).clicked() {
                            self.about_dialog = true;
                        }
                    });
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

        // Show IMEI popup if available
        if let Some(imei) = &self.imei_popup {
            let imei_clone = imei.clone();
            egui::Window::new("Device IMEI")
                .collapsible(false)
                .resizable(false)
                .fixed_size(egui::vec2(260.0, 120.0))
                .frame(egui::Frame::window(&egui::Style::default()).corner_radius(egui::CornerRadius::same(0)))
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("📱 Device IMEI").size(12.0));
                    ui.separator();
                    ui.label(egui::RichText::new(&imei_clone).size(22.0).monospace());
                    ui.separator();
                    if ui.add(egui::Button::new(egui::RichText::new("Close").size(12.0))).clicked() {
                        self.imei_popup = None;
                    }
                });
        }

        // Show Display Info popup if available
        if let Some(display_info) = &self.display_popup {
            let display_clone = display_info.clone();
            egui::Window::new("Display Information")
                .collapsible(false)
                .resizable(true)
                .default_size(egui::vec2(400.0, 300.0))
                .frame(egui::Frame::window(&egui::Style::default()).corner_radius(egui::CornerRadius::same(0)))
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("📺 Display Information").size(12.0));
                    ui.separator();
                    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                        ui.label(egui::RichText::new(&display_clone).size(11.0).monospace());
                    });
                    ui.separator();
                    if ui.add(egui::Button::new(egui::RichText::new("Close").size(12.0))).clicked() {
                        self.display_popup = None;
                    }
                });
        }

        // Show Battery Info popup if available
        if let Some(battery_info) = &self.battery_popup {
            let battery_clone = battery_info.clone();
            egui::Window::new("Battery Information")
                .collapsible(false)
                .resizable(true)
                .default_size(egui::vec2(350.0, 250.0))
                .frame(egui::Frame::window(&egui::Style::default()).corner_radius(egui::CornerRadius::same(0)))
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("🔋 Battery Information").size(12.0));
                    ui.separator();
                    egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                        ui.label(egui::RichText::new(&battery_clone).size(11.0).monospace());
                    });
                    ui.separator();
                    if ui.add(egui::Button::new(egui::RichText::new("Close").size(12.0))).clicked() {
                        self.battery_popup = None;
                    }
                });
        }

        // Show Screen Recording Dialog if available
        if self.screenrecord_dialog {
            egui::Window::new("Screen Recording Settings")
                .collapsible(false)
                .resizable(false)
                .fixed_size(egui::vec2(300.0, 200.0))
                .frame(egui::Frame::window(&egui::Style::default()).corner_radius(egui::CornerRadius::same(0)))
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("🎥 Screen Recording Settings").size(12.0));
                    ui.separator();
                    
                    ui.horizontal(|ui| {
                        ui.label("Duration (seconds):");
                        ui.add(egui::DragValue::new(&mut self.screenrecord_duration).range(1..=180).speed(1));
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label("Bitrate (KB/s):");
                        ui.add(egui::DragValue::new(&mut self.screenrecord_bitrate).range(100..=10000).speed(100));
                    });
                    
                    ui.separator();
                    
                    ui.horizontal(|ui| {
                        if ui.add(egui::Button::new(egui::RichText::new("Start Recording").size(12.0))).clicked() {
                            if let (Some(adb_bridge), Some(device)) = (self.adb_bridge.as_ref(), self.device_list.selected_device()) {
                                // Start screen recording with custom settings
                                let status = std::process::Command::new(adb_bridge.path())
                                    .args([
                                        "-s",
                                        &device.identifier,
                                        "shell",
                                        "screenrecord",
                                        "/sdcard/video.mp4",
                                        "--time-limit",
                                        &self.screenrecord_duration.to_string(),
                                        "--bit-rate",
                                        &(self.screenrecord_bitrate * 1000).to_string(),
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
                                                self.status_message = format!("Screenrecord saved to {}", file_path.display());
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
                                self.screenrecord_dialog = false;
                            } else {
                                self.status_message = "No device selected or ADB not configured".to_string();
                            }
                        }
                        
                        if ui.add(egui::Button::new(egui::RichText::new("Cancel").size(12.0))).clicked() {
                            self.screenrecord_dialog = false;
                        }
                    });
                });
        }

        // Show About Dialog if available
        if self.about_dialog {
            egui::Area::new("about_dialog".into())
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style())
                        .show(ui, |ui| {
                            ui.set_width(280.0);
                            ui.set_height(180.0);
                            
                            ui.vertical_centered(|ui| {
                                ui.add_space(8.0);
                                
                                // App icon
                                if let Ok(img) = image::load_from_memory(ICON_PNG) {
                                    let img = img.to_rgba8();
                                    let (w, h) = img.dimensions();
                                    let color_image = egui::ColorImage::from_rgba_unmultiplied(
                                        [w as usize, h as usize],
                                        img.as_raw(),
                                    );
                                    let texture_id = ui.ctx().load_texture(
                                        "about_icon",
                                        color_image,
                                        egui::TextureOptions::default()
                                    );
                                    ui.add(egui::Image::new(&texture_id).fit_to_exact_size(egui::vec2(48.0, 48.0)));
                                    ui.add_space(8.0);
                                } else {
                                    // Fallback to emoji if icon not found
                                    ui.label(egui::RichText::new("📱").size(32.0));
                                }
                                
                                // App name and version
                                ui.label(egui::RichText::new("DroidView").size(20.0).strong());
                                ui.label(egui::RichText::new("(droid_view)").size(10.0).color(Color32::GRAY));
                                ui.label(egui::RichText::new("Version 0.1.1").size(12.0));
                                
                                ui.add_space(8.0);
                                
                                // Author
                                ui.label(egui::RichText::new("Author: Genxster1998").size(11.0));
                                
                                ui.add_space(4.0);
                                
                                // Copyright
                                ui.label(egui::RichText::new("© 2024 Genxster1998").size(10.0).color(Color32::GRAY));
                                
                                ui.add_space(8.0);
                                
                                // Website
                                ui.vertical_centered(|ui| {
                                    ui.label(egui::RichText::new("Website:").size(10.0));
                                    if ui.link(egui::RichText::new("github.com/Genxster1998/DroidView").size(10.0).color(Color32::BLUE)).clicked() {
                                        // Open URL in default browser
                                        let _ = std::process::Command::new("open")
                                            .arg("https://github.com/Genxster1998/DroidView")
                                            .output();
                                    }
                                });
                                
                                ui.add_space(12.0);
                                
                                // Close button
                                if ui.add(egui::Button::new(egui::RichText::new("Close").size(11.0)).min_size(egui::vec2(60.0, 24.0))).clicked() {
                                    self.about_dialog = false;
                                }
                            });
                        });
                });
        }

        // Show Uninstall App Dialog if available
        if self.uninstall_dialog {
            egui::Window::new("Uninstall Application")
                .collapsible(false)
                .resizable(true)
                .default_size(egui::vec2(400.0, 500.0))
                .frame(egui::Frame::window(&egui::Style::default()).corner_radius(egui::CornerRadius::same(0)))
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("🗑️ Uninstall Application").size(12.0));
                    ui.separator();
                    
                    if self.loading_apps {
                        ui.vertical_centered(|ui| {
                            ui.add_space(20.0);
                            ui.label("Loading app list...");
                            ui.add(egui::Spinner::new().size(20.0));
                            ui.add_space(20.0);
                        });
                    } else if self.app_list.is_empty() {
                        ui.label("No apps found or failed to load app list.");
                    } else {
                        ui.label(format!("Found {} apps:", self.app_list.len()));
                        ui.separator();
                        
                        // App selection with checkboxes
                        egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                            for (package_name, _) in &self.app_list {
                                let is_selected = self.selected_apps.contains(package_name);
                                let mut checked = is_selected;
                                
                                ui.horizontal(|ui| {
                                    if ui.checkbox(&mut checked, "").clicked() {
                                        if checked {
                                            self.selected_apps.insert(package_name.clone());
                                        } else {
                                            self.selected_apps.remove(package_name);
                                        }
                                    }
                                    
                                    ui.label(package_name);
                                });
                            }
                        });
                        
                        ui.separator();
                        
                        // Selection summary
                        if !self.selected_apps.is_empty() {
                            ui.label(format!("Selected {} app(s)", self.selected_apps.len()));
                        }
                        
                        // Uninstall buttons
                        ui.horizontal(|ui| {
                            if ui.add(egui::Button::new(egui::RichText::new("Uninstall Selected").size(12.0))).clicked() {
                                if !self.selected_apps.is_empty() {
                                    if let (Some(adb_bridge), Some(device)) = (
                                        self.adb_bridge.as_ref(), 
                                        self.device_list.selected_device()
                                    ) {
                                        let mut success_count = 0;
                                        let mut failed_count = 0;
                                        
                                        for package_name in &self.selected_apps {
                                            // Uninstall the selected app
                                            let status = std::process::Command::new(adb_bridge.path())
                                                .args([
                                                    "-s",
                                                    &device.identifier,
                                                    "uninstall",
                                                    package_name,
                                                ])
                                                .status();
                                            
                                            match status {
                                                Ok(s) if s.success() => {
                                                    success_count += 1;
                                                }
                                                _ => {
                                                    failed_count += 1;
                                                }
                                            }
                                        }
                                        
                                        // Remove successfully uninstalled apps from list
                                        self.app_list.retain(|(package, _)| !self.selected_apps.contains(package));
                                        
                                        if failed_count == 0 {
                                            self.status_message = format!("Successfully uninstalled {} app(s)", success_count);
                                        } else {
                                            self.status_message = format!("Uninstalled {} app(s), {} failed", success_count, failed_count);
                                        }
                                        
                                        self.selected_apps.clear();
                                    } else {
                                        self.status_message = "No device selected or ADB not configured".to_string();
                                    }
                                } else {
                                    self.status_message = "Please select at least one app to uninstall".to_string();
                                }
                            }
                            
                            if ui.add(egui::Button::new(egui::RichText::new("Select All").size(12.0))).clicked() {
                                self.selected_apps.clear();
                                for (package_name, _) in &self.app_list {
                                    self.selected_apps.insert(package_name.clone());
                                }
                            }
                            
                            if ui.add(egui::Button::new(egui::RichText::new("Clear Selection").size(12.0))).clicked() {
                                self.selected_apps.clear();
                            }
                            
                            if ui.add(egui::Button::new(egui::RichText::new("Close").size(12.0))).clicked() {
                                self.uninstall_dialog = false;
                                self.selected_apps.clear();
                            }
                        });
                    }
                });
        }

        // Show Disable App Dialog if available
        if self.disable_dialog {
            egui::Window::new("Disable Application")
                .collapsible(false)
                .resizable(true)
                .default_size(egui::vec2(400.0, 500.0))
                .frame(egui::Frame::window(&egui::Style::default()).corner_radius(egui::CornerRadius::same(0)))
                .pivot(egui::Align2::CENTER_CENTER)
                .show(ctx, |ui| {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("🚫 Disable Application").size(12.0));
                    ui.separator();
                    
                    if self.loading_disable_apps {
                        ui.vertical_centered(|ui| {
                            ui.add_space(20.0);
                            ui.label("Loading app list...");
                            ui.add(egui::Spinner::new().size(20.0));
                            ui.add_space(20.0);
                        });
                    } else if self.disable_app_list.is_empty() {
                        ui.label("No apps found or failed to load app list.");
                    } else {
                        ui.label(format!("Found {} enabled apps:", self.disable_app_list.len()));
                        ui.separator();
                        
                        // App selection with checkboxes
                        egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                            for (package_name, _) in &self.disable_app_list {
                                let is_selected = self.selected_disable_apps.contains(package_name);
                                let mut checked = is_selected;
                                
                                ui.horizontal(|ui| {
                                    if ui.checkbox(&mut checked, "").clicked() {
                                        if checked {
                                            self.selected_disable_apps.insert(package_name.clone());
                                        } else {
                                            self.selected_disable_apps.remove(package_name);
                                        }
                                    }
                                    
                                    ui.label(package_name);
                                });
                            }
                        });
                        
                        ui.separator();
                        
                        // Selection summary
                        if !self.selected_disable_apps.is_empty() {
                            ui.label(format!("Selected {} app(s)", self.selected_disable_apps.len()));
                        }
                        
                        // Disable buttons
                        ui.horizontal(|ui| {
                            if ui.add(egui::Button::new(egui::RichText::new("Disable Selected").size(12.0))).clicked() {
                                if !self.selected_disable_apps.is_empty() {
                                    if let (Some(adb_bridge), Some(device)) = (
                                        self.adb_bridge.as_ref(), 
                                        self.device_list.selected_device()
                                    ) {
                                        let mut success_count = 0;
                                        let mut failed_count = 0;
                                        
                                        for package_name in &self.selected_disable_apps {
                                            // Disable the selected app for user 0
                                            let status = std::process::Command::new(adb_bridge.path())
                                                .args([
                                                    "-s",
                                                    &device.identifier,
                                                    "shell",
                                                    "pm disable-user --user 0",
                                                    package_name,
                                                ])
                                                .status();
                                            
                                            match status {
                                                Ok(s) if s.success() => {
                                                    success_count += 1;
                                                }
                                                _ => {
                                                    failed_count += 1;
                                                }
                                            }
                                        }
                                        
                                        // Remove successfully disabled apps from list
                                        self.disable_app_list.retain(|(package, _)| !self.selected_disable_apps.contains(package));
                                        
                                        if failed_count == 0 {
                                            self.status_message = format!("Successfully disabled {} app(s)", success_count);
                                        } else {
                                            self.status_message = format!("Disabled {} app(s), {} failed", success_count, failed_count);
                                        }
                                        
                                        self.selected_disable_apps.clear();
                                    } else {
                                        self.status_message = "No device selected or ADB not configured".to_string();
                                    }
                                } else {
                                    self.status_message = "Please select at least one app to disable".to_string();
                                }
                            }
                            
                            if ui.add(egui::Button::new(egui::RichText::new("Select All").size(12.0))).clicked() {
                                self.selected_disable_apps.clear();
                                for (package_name, _) in &self.disable_app_list {
                                    self.selected_disable_apps.insert(package_name.clone());
                                }
                            }
                            
                            if ui.add(egui::Button::new(egui::RichText::new("Clear Selection").size(12.0))).clicked() {
                                self.selected_disable_apps.clear();
                            }
                            
                            if ui.add(egui::Button::new(egui::RichText::new("Close").size(12.0))).clicked() {
                                self.disable_dialog = false;
                                self.selected_disable_apps.clear();
                            }
                        });
                    }
                });
        }

        self.update_background_tasks();
        self.settings_window.show(ctx);
    }
}
