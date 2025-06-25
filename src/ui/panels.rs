use egui::Ui;

pub enum BottomPanelAction {
    None,
    RefreshDevices,
    RestartAdb,
    OpenSettings,
}

pub enum ToolkitAction {
    None,
    Screenshot,
    RecordScreen,
    InstallApk,
    OpenShell,
    ShowImei,
    DisplayInfo,
    BatteryInfo,
    UninstallApp,
    DisableApp,
}

pub enum SwipeAction {
    Up,
    Down,
    Left,
    Right,
}

pub struct SwipePanel {
    pub visible: bool,
}

pub struct ToolkitPanel {
    pub visible: bool,
}

pub struct BottomPanel {
    pub visible: bool,
}

pub struct WirelessAdbPanel {
    visible: bool,
    tcpip_ip: String,
    tcpip_port: String,
    pairing_ip: String,
    pairing_port: String,
    pairing_code: String,
    selected_device: Option<String>,
    config: Option<std::sync::Arc<tokio::sync::Mutex<crate::config::AppConfig>>>,
}

impl Default for SwipePanel {
    fn default() -> Self {
        Self::new()
    }
}

impl SwipePanel {
    pub fn new() -> Self {
        Self { visible: true }
    }

    pub fn show(&mut self, ui: &mut Ui) -> Option<SwipeAction> {
        if !self.visible {
            return None;
        }

        let mut action = None;

        ui.group(|ui| {
            ui.heading("Swipe Controls");

            ui.horizontal(|ui| {
                if ui.button("▲ Swipe Up").clicked() {
                    action = Some(SwipeAction::Up);
                }
                if ui.button("▼ Swipe Down").clicked() {
                    action = Some(SwipeAction::Down);
                }
            });

            ui.horizontal(|ui| {
                if ui.button("◀ Swipe Left").clicked() {
                    action = Some(SwipeAction::Left);
                }
                if ui.button("▶ Swipe Right").clicked() {
                    action = Some(SwipeAction::Right);
                }
            });
        });
        action
    }
}

impl Default for ToolkitPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolkitPanel {
    pub fn new() -> Self {
        Self { visible: true }
    }

    pub fn show(&mut self, ui: &mut Ui, loading: &ToolkitLoadingState) -> ToolkitAction {
        if !self.visible {
            return ToolkitAction::None;
        }

        let mut action = ToolkitAction::None;

        ui.group(|ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Toolkit");
            });

            ui.vertical_centered(|ui| {
                // Screenshot button
                ui.vertical_centered(|ui| {
                    if ui.add(
                        egui::Button::new(
                            egui::RichText::new("📸 Screenshot").size(13.0)
                        ).min_size(egui::vec2(120.0, 28.0))
                    ).clicked() {
                        action = ToolkitAction::Screenshot;
                    }
                });

                // Record Screen button
                ui.vertical_centered(|ui| {
                    if ui.add(
                        egui::Button::new(
                            egui::RichText::new("🎥 Record Screen").size(13.0)
                        ).min_size(egui::vec2(120.0, 28.0))
                    ).clicked() {
                        action = ToolkitAction::RecordScreen;
                    }
                });

                // Install APK button
                ui.vertical_centered(|ui| {
                    if ui.add(
                        egui::Button::new(
                            egui::RichText::new("📱 Install APK").size(13.0)
                        ).min_size(egui::vec2(120.0, 28.0))
                    ).clicked() {
                        action = ToolkitAction::InstallApk;
                    }
                });

                // ADB Shell button
                ui.vertical_centered(|ui| {
                    if ui.add(
                        egui::Button::new(
                            egui::RichText::new("💻 ADB Shell").size(13.0)
                        ).min_size(egui::vec2(120.0, 28.0))
                    ).clicked() {
                        action = ToolkitAction::OpenShell;
                    }
                });

                // Show IMEI button with spinner
                ui.vertical_centered(|ui| {
                    if ui.add(
                        egui::Button::new(
                            egui::RichText::new("📱 Show IMEI").size(13.0)
                        ).min_size(egui::vec2(120.0, 28.0))
                    ).clicked() {
                        action = ToolkitAction::ShowImei;
                    }
                    if loading.show_imei {
                        ui.add(egui::Spinner::new().size(16.0));
                    }
                });

                // Show Display Info button with spinner
                ui.vertical_centered(|ui| {
                    if ui.add(
                        egui::Button::new(
                            egui::RichText::new("📺 Display Info").size(13.0)
                        ).min_size(egui::vec2(120.0, 28.0))
                    ).clicked() {
                        action = ToolkitAction::DisplayInfo;
                    }
                    if loading.display_info {
                        ui.add(egui::Spinner::new().size(16.0));
                    }
                });

                // Show Battery Info button with spinner
                ui.vertical_centered(|ui| {
                    if ui.add(
                        egui::Button::new(
                            egui::RichText::new("🔋 Battery Info").size(13.0)
                        ).min_size(egui::vec2(120.0, 28.0))
                    ).clicked() {
                        action = ToolkitAction::BatteryInfo;
                    }
                    if loading.battery_info {
                        ui.add(egui::Spinner::new().size(16.0));
                    }
                });

                // Show Uninstall App button with spinner
                ui.vertical_centered(|ui| {
                    if ui.add(
                        egui::Button::new(
                            egui::RichText::new("🗑️ Uninstall App").size(13.0)
                        ).min_size(egui::vec2(120.0, 28.0))
                    ).clicked() {
                        action = ToolkitAction::UninstallApp;
                    }
                    if loading.uninstall_app {
                        ui.add(egui::Spinner::new().size(16.0));
                    }
                });

                // Show Disable App button with spinner
                ui.vertical_centered(|ui| {
                    if ui.add(
                        egui::Button::new(
                            egui::RichText::new("🚫 Disable App").size(13.0)
                        ).min_size(egui::vec2(120.0, 28.0))
                    ).clicked() {
                        action = ToolkitAction::DisableApp;
                    }
                    if loading.disable_app {
                        ui.add(egui::Spinner::new().size(16.0));
                    }
                });
            });
        });
        action
    }
}

// Helper struct for loading states
pub struct ToolkitLoadingState {
    pub screenshot: bool,
    pub record_screen: bool,
    pub install_apk: bool,
    pub open_shell: bool,
    pub show_imei: bool,
    pub display_info: bool,
    pub battery_info: bool,
    pub uninstall_app: bool,
    pub disable_app: bool,
}

impl Default for BottomPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl BottomPanel {
    pub fn new() -> Self {
        Self { visible: true }
    }

    pub fn show(&mut self, ui: &mut Ui) -> BottomPanelAction {
        if !self.visible {
            return BottomPanelAction::None;
        }

        let mut action = BottomPanelAction::None;

        ui.group(|ui| {
            ui.heading("Quick Actions");

            ui.horizontal(|ui| {
                if ui.button("🔄 Refresh Devices").clicked() {
                    action = BottomPanelAction::RefreshDevices;
                }

                if ui.button("🔄 Restart ADB").clicked() {
                    action = BottomPanelAction::RestartAdb;
                }

                if ui.button("🔧 Settings").clicked() {
                    action = BottomPanelAction::OpenSettings;
                }
            });
        });

        action
    }
}

impl Default for WirelessAdbPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl WirelessAdbPanel {
    pub fn new() -> Self {
        Self {
            visible: true,
            tcpip_ip: String::new(),
            tcpip_port: "5555".to_string(),
            pairing_ip: String::new(),
            pairing_port: "5555".to_string(),
            pairing_code: String::new(),
            selected_device: None,
            config: None,
        }
    }

    pub fn set_config(&mut self, config: std::sync::Arc<tokio::sync::Mutex<crate::config::AppConfig>>) {
        self.config = Some(config.clone());
        // Load remembered IPs
        if let Ok(config_lock) = config.try_lock() {
            self.tcpip_ip = config_lock.wireless_adb.last_tcpip_ip.clone();
            self.tcpip_port = config_lock.wireless_adb.last_tcpip_port.clone();
            self.pairing_ip = config_lock.wireless_adb.last_pairing_ip.clone();
            self.pairing_port = config_lock.wireless_adb.last_pairing_port.clone();
        }
    }

    fn save_ips(&mut self) {
        if let Some(config) = &self.config {
            if let Ok(mut config_lock) = config.try_lock() {
                config_lock.wireless_adb.last_tcpip_ip = self.tcpip_ip.clone();
                config_lock.wireless_adb.last_tcpip_port = self.tcpip_port.clone();
                config_lock.wireless_adb.last_pairing_ip = self.pairing_ip.clone();
                config_lock.wireless_adb.last_pairing_port = self.pairing_port.clone();
                // Save config
                let _ = config_lock.save();
            }
        }
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        _adb_bridge: Option<&crate::bridge::AdbBridge>,
        devices: &[crate::device::Device],
    ) -> Option<WirelessAdbAction> {
        if !self.visible {
            return None;
        }

        let mut action = None;

        ui.group(|ui| {
            ui.heading("Wireless ADB");

            // TCP/IP Connection Section
            ui.group(|ui| {
                ui.heading("Direct TCP/IP Connection");

                ui.horizontal(|ui| {
                    ui.label("IP Address:");
                    ui.text_edit_singleline(&mut self.tcpip_ip);
                });

                ui.horizontal(|ui| {
                    ui.label("Port:");
                    ui.text_edit_singleline(&mut self.tcpip_port);
                });

                if ui.button("🔗 Connect").clicked() {
                    if let Ok(port) = self.tcpip_port.parse::<u16>() {
                        self.save_ips(); // Save IPs when connecting
                        action = Some(WirelessAdbAction::Connect {
                            ip: self.tcpip_ip.clone(),
                            port,
                        });
                    }
                }
            });

            ui.separator();

            // TCP/IP Setup Section (for connected devices)
            ui.group(|ui| {
                ui.heading("Enable TCP/IP on Device");

                if devices.is_empty() {
                    ui.label("No devices connected");
                } else {
                    // Device selection dropdown
                    egui::ComboBox::from_id_source("device_select")
                        .selected_text(
                            self.selected_device
                                .as_ref()
                                .unwrap_or(&"Select a device".to_string()),
                        )
                        .show_ui(ui, |ui| {
                            for device in devices {
                                if device.is_usable() {
                                    ui.selectable_value(
                                        &mut self.selected_device,
                                        Some(device.identifier.clone()),
                                        &device.model,
                                    );
                                }
                            }
                        });

                    if let Ok(port) = self.tcpip_port.parse::<u16>() {
                        if ui.button("🌐 Enable TCP/IP").clicked() {
                            if let Some(device_id) = &self.selected_device {
                                action = Some(WirelessAdbAction::EnableTcpip {
                                    device_id: device_id.clone(),
                                    port,
                                });
                            }
                        }
                    }
                }
            });

            ui.separator();

            // Pairing Section
            ui.group(|ui| {
                ui.heading("Pair via Code");

                ui.horizontal(|ui| {
                    ui.label("IP Address:");
                    ui.text_edit_singleline(&mut self.pairing_ip);
                });

                ui.horizontal(|ui| {
                    ui.label("Port:");
                    ui.text_edit_singleline(&mut self.pairing_port);
                });

                ui.horizontal(|ui| {
                    ui.label("Pairing Code:");
                    ui.text_edit_singleline(&mut self.pairing_code);
                });

                if ui.button("🔐 Pair").clicked() {
                    if let Ok(port) = self.pairing_port.parse::<u16>() {
                        self.save_ips(); // Save IPs when pairing
                        action = Some(WirelessAdbAction::Pair {
                            ip: self.pairing_ip.clone(),
                            port,
                            code: self.pairing_code.clone(),
                        });
                    }
                }
            });
        });

        action
    }
}

pub enum WirelessAdbAction {
    Connect { ip: String, port: u16 },
    EnableTcpip { device_id: String, port: u16 },
    Pair { ip: String, port: u16, code: String },
}
