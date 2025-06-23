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
    FileManager,
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
}

impl SwipePanel {
    pub fn new() -> Self {
        Self { visible: true }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        if !self.visible {
            return;
        }

        ui.group(|ui| {
            ui.heading("Swipe Controls");
            
            ui.horizontal(|ui| {
                if ui.button("▲ Swipe Up").clicked() {
                    // TODO: Implement swipe up
                }
                if ui.button("▼ Swipe Down").clicked() {
                    // TODO: Implement swipe down
                }
            });
            
            ui.horizontal(|ui| {
                if ui.button("◀ Swipe Left").clicked() {
                    // TODO: Implement swipe left
                }
                if ui.button("▶ Swipe Right").clicked() {
                    // TODO: Implement swipe right
                }
            });
        });
    }
}

impl ToolkitPanel {
    pub fn new() -> Self {
        Self { visible: true }
    }

    pub fn show(&mut self, ui: &mut Ui) -> ToolkitAction {
        if !self.visible {
            return ToolkitAction::None;
        }

        let mut action = ToolkitAction::None;

        ui.group(|ui| {
            ui.heading("Toolkit");
            
            ui.vertical(|ui| {
                if ui.button("📸 Screenshot").clicked() {
                    action = ToolkitAction::Screenshot;
                }
                
                if ui.button("🎥 Record Screen").clicked() {
                    action = ToolkitAction::RecordScreen;
                }
                
                if ui.button("📱 Install APK").clicked() {
                    action = ToolkitAction::InstallApk;
                }
                
                if ui.button("📁 File Manager").clicked() {
                    action = ToolkitAction::FileManager;
                }
            });
        });
        action
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
        }
    }

    pub fn show(&mut self, ui: &mut Ui, _adb_bridge: Option<&crate::bridge::AdbBridge>, devices: &[crate::device::Device]) -> Option<WirelessAdbAction> {
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
                                .unwrap_or(&"Select a device".to_string())
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

                    if let Some(port) = self.tcpip_port.parse::<u16>().ok() {
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