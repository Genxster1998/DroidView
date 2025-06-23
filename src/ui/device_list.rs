use crate::device::{Device, DeviceStatus};
use egui::{Color32, RichText, Ui};

pub struct DeviceList {
    devices: Vec<Device>,
    selected_device: Option<usize>,
}

impl DeviceList {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            selected_device: None,
        }
    }

    pub fn update_devices(&mut self, devices: Vec<Device>) {
        self.devices = devices;
        
        // Reset selection if device list is empty
        if self.devices.is_empty() {
            self.selected_device = None;
            return;
        }
        // Auto-select first usable device if none selected
        if self.selected_device.is_none() {
            if let Some(index) = self.devices.iter().position(|d| d.is_usable()) {
                self.selected_device = Some(index);
            }
        } else if let Some(i) = self.selected_device {
            // If the previously selected index is now out of bounds, reset
            if i >= self.devices.len() {
                self.selected_device = None;
            }
        }
    }

    pub fn selected_device(&self) -> Option<&Device> {
        match self.selected_device {
            Some(i) if i < self.devices.len() => Some(&self.devices[i]),
            _ => None,
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ui.heading("Connected Devices");
        
        if self.devices.is_empty() {
            ui.label(RichText::new("No devices found").color(Color32::GRAY));
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (index, device) in self.devices.iter().enumerate() {
                let is_selected = self.selected_device == Some(index);
                let is_usable = device.is_usable();
                
                let text = if is_usable {
                    RichText::new(&device.model)
                } else {
                    RichText::new(&device.model).color(Color32::GRAY)
                };

                let status_text = match &device.status {
                    DeviceStatus::Device => RichText::new("✅ Connected").color(Color32::GREEN),
                    DeviceStatus::Offline => RichText::new("❌ Offline").color(Color32::RED),
                    DeviceStatus::Unauthorized => RichText::new("⚠️ Unauthorized").color(Color32::YELLOW),
                    DeviceStatus::NoPermission => RichText::new("🚫 No Permission").color(Color32::RED),
                    DeviceStatus::Unknown(s) => RichText::new(format!("❓ {}", s)).color(Color32::GRAY),
                };

                ui.horizontal(|ui| {
                    if ui.selectable_label(is_selected, text).clicked() && is_usable {
                        self.selected_device = Some(index);
                    }
                    
                    ui.label(status_text);
                });

                if is_selected {
                    ui.indent("device_info", |ui| {
                        ui.label(format!("ID: {}", device.identifier));
                        ui.label(format!("Product: {}", device.product));
                        ui.label(format!("Model: {}", device.model));
                        ui.label(format!("Device: {}", device.device));
                    });
                }
            }
        });
    }
} 